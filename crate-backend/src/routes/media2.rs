use std::cmp::Ordering;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing, Json,
};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{application::Scope, sync::MessageSync};
use common::v2::types::media::{
    Media, MediaClone, MediaCreate, MediaCreated, MediaDoneParams, MediaPatch, MediaSearch,
};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::error::{Error, Result};
use crate::types::MediaId;
use crate::ServerState;

use super::util::Auth;

/// Media create
///
/// Create a new url to upload media to. Use the media upload endpoint for
/// actually uploading media. Media not referenced/used in other api calls will
/// be removed after a period of time.
#[utoipa::path(
    post,
    path = "/media",
    tags = ["media", "badge.scope.full"],
    responses(
        (status = StatusCode::CREATED, description = "Create media success", body = MediaCreated)
    )
)]
async fn media_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MediaCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    json.validate()?;
    match &json.source {
        common::v2::types::media::MediaCreateSource::Upload { size, .. } => {
            if *size > Some(s.config.media_max_size) {
                return Err(Error::TooBig);
            }

            let media_id = MediaId::new();
            let srv = s.services();
            srv.media
                .create_upload(media_id, auth.user.id, json.clone().into())
                .await?;
            let upload_url = Some(
                s.config()
                    .api_url
                    .join(&format!("/api/v1/internal/media-upload/{media_id}"))?,
            );
            let res = MediaCreated {
                media_id,
                upload_url,
            };
            let mut res_headers = HeaderMap::new();
            if let Some(sz) = size {
                res_headers.insert("upload-length", (*sz).into());
            }
            res_headers.insert("upload-offset", 0.into());
            Ok((StatusCode::CREATED, res_headers, Json(res)))
        }
        common::v2::types::media::MediaCreateSource::Download { size, .. } => {
            if size.is_some_and(|sz| sz > s.config.media_max_size) {
                return Err(Error::TooBig);
            }

            let srv = s.services();
            let media = srv
                .media
                .import_from_url(auth.user.id, json.clone().into())
                .await?;
            let mut headers = HeaderMap::new();
            headers.insert("content-length", media.size.into());
            let res = MediaCreated {
                media_id: media.id,
                upload_url: None,
            };
            Ok((StatusCode::CREATED, headers, Json(res)))
        }
    }
}

/// Media patch
///
/// Edit properties about some piece of media
#[utoipa::path(
    patch,
    path = "/media/{media_id}",
    tags = ["media", "badge.scope.full"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NOT_MODIFIED, description = "Not modified"),
        (status = OK, body = Media, description = "Success"),
    )
)]
async fn media_patch(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MediaPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    json.validate()?;

    let media = s.data().media2_select(media_id).await?;
    if media.deleted_at.is_some() {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownMedia,
        )));
    }
    if media.user_id != Some(auth.user.id) {
        // NOTE: should i return UnknownMedia here to prevent leaking info?
        return Err(Error::MissingPermissions);
    }

    s.data().media2_update(media_id, json).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Media done
///
/// finishes a media upload and begins processing
#[utoipa::path(
    put,
    path = "/media/{media_id}/done",
    tags = ["media", "badge.scope.full"],
    params(("media_id", description = "Media id")),
    request_body = MediaDoneParams,
    responses(
        (status = OK, body = Media, description = "Success"),
        (status = ACCEPTED, description = "Processing in background"),
    ),
)]
async fn media_done(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(params): Json<MediaDoneParams>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let mut up =
        srv.media
            .uploads
            .get_mut(&media_id)
            .ok_or(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownMedia,
            )))?;
    if up.user_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let source_size = up
        .create
        .source
        .size()
        .expect("can only patch source upload");
    match up.current_size.cmp(&source_size) {
        Ordering::Greater => {
            s.services().media.uploads.remove(&media_id);
            Err(Error::TooBig)
        }
        Ordering::Equal => {
            up.temp_writer.flush().await?;
            drop(up);
            let (_, up) = s
                .services()
                .media
                .uploads
                .remove(&media_id)
                .expect("it was there a few milliseconds ago");
            let filename = match &up.create.source {
                common::v2::types::media::MediaCreateSource::Upload { filename, .. } => {
                    filename.to_owned()
                }
                common::v2::types::media::MediaCreateSource::Download { .. } => {
                    panic!("can only patch upload")
                }
            };

            if params.process_async {
                let state = s.clone();
                let user_id = auth.user.id;
                let mut headers = HeaderMap::new();
                headers.insert("upload-offset", source_size.into());
                headers.insert("upload-length", source_size.into());
                tokio::spawn(async move {
                    match state
                        .services()
                        .media
                        .process_upload(up, media_id, user_id, &filename)
                        .await
                    {
                        Ok(media) => {
                            debug!("finished processing media asynchronously");
                            let msg = MessageSync::MediaProcessed {
                                media: media.clone(),
                            };
                            if let Err(e) = state.broadcast(msg) {
                                error!("failed to broadcast MediaProcessed: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("failed to process media asynchronously: {}", e);
                        }
                    }
                });
                Ok((StatusCode::ACCEPTED, headers, Json(None)))
            } else {
                let media = s
                    .services()
                    .media
                    .process_upload(up, media_id, auth.user.id, &filename)
                    .await?;
                let mut headers = HeaderMap::new();
                headers.insert("upload-offset", media.size.into());
                headers.insert("upload-length", media.size.into());
                Ok((StatusCode::OK, headers, Json(Some(media))))
            }
        }
        Ordering::Less => {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.current_size.into());
            headers.insert("upload-length", source_size.into());
            Ok((StatusCode::NO_CONTENT, headers, Json(None)))
        }
    }
}

/// Media upload
///
/// Always returns immediately, but will automatically begin processing media in
/// the background.
async fn media_upload(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let mut up =
        srv.media
            .uploads
            .get_mut(&media_id)
            .ok_or(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownMedia,
            )))?;
    if up.user_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let stat = up.temp_file.metadata().await?;
    let current_size = stat.len();
    let current_off: u64 = headers
        .get("upload-offset")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    let part_length: u64 = headers
        .get("content-length")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    if current_size != current_off {
        return Err(Error::CantOverwrite);
    }
    let source_size = up
        .create
        .source
        .size()
        .expect("can only patch source upload");
    if current_size + part_length > source_size {
        return Err(Error::TooBig);
    }
    up.seek(current_off).await?;
    let mut stream = body.into_data_stream();

    while let Some(chunk) = stream.next().await {
        if let Err(err) = up.write(&chunk?).await {
            srv.media.uploads.remove(&media_id);
            return Err(err);
        };
    }

    match up.current_size.cmp(&source_size) {
        Ordering::Greater => {
            s.services().media.uploads.remove(&media_id);
            Err(Error::TooBig)
        }
        Ordering::Equal => {
            up.temp_writer.flush().await?;

            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.current_size.into());
            headers.insert("upload-length", source_size.into());

            let state = s.clone();
            let user_id = auth.user.id;
            tokio::spawn(async move {
                let up = match state.services().media.uploads.remove(&media_id) {
                    Some((_, up)) => up,
                    None => {
                        error!("upload was removed before processing");
                        return;
                    }
                };
                let filename = match &up.create.source {
                    common::v2::types::media::MediaCreateSource::Upload { filename, .. } => {
                        filename.to_owned()
                    }
                    common::v2::types::media::MediaCreateSource::Download { .. } => {
                        panic!("can only patch upload")
                    }
                };
                match state
                    .services()
                    .media
                    .process_upload(up, media_id, user_id, &filename)
                    .await
                {
                    Ok(media) => {
                        debug!("finished processing media from upload");
                        let msg = MessageSync::MediaProcessed {
                            media: media.clone(),
                        };
                        if let Err(e) = state.broadcast(msg) {
                            error!("failed to broadcast MediaProcessed: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("failed to process media from upload: {}", e);
                    }
                }
            });

            Ok((StatusCode::NO_CONTENT, headers))
        }
        Ordering::Less => {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.current_size.into());
            headers.insert("upload-length", source_size.into());
            Ok((StatusCode::NO_CONTENT, headers))
        }
    }
}

/// Media get
///
/// Get a piece of media. Currently, all media is public (though this may change in the future).
#[utoipa::path(
    get,
    path = "/media/{media_id}",
    tags = ["media", "badge.scope.full"],
    params(("media_id", description = "Media id")),
    responses(
        (status = OK, body = Media, description = "Success"),
    )
)]
async fn media_get(
    Path(media_id): Path<MediaId>,
    _auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let media = s.data().media2_select(media_id).await?;
    Ok(Json(media))
}

/// Media check
///
/// Get headers useful for resuming an upload
// NOTE: im not sure why this is commented out? presumably theres some issue, i'll fix it later
// #[utoipa::path(
//     head,
//     path = "/media/{media_id}",
//     tags = ["media", "badge.internal"],
//     params(("media_id", description = "Media id")),
//     responses(
//         (status = NO_CONTENT, description = "no content", headers(("upload-length" = u64), ("upload-offset" = u64))),
//     )
// )]
async fn media_check(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if let Some(up) = s.services().media.uploads.get_mut(&media_id) {
        if up.user_id == auth.user.id {
            let mut headers = HeaderMap::new();
            headers.insert("upload-offset", up.temp_file.metadata().await?.len().into());
            headers.insert(
                "upload-length",
                up.create
                    .source
                    .size()
                    .expect("can only check media where source = upload")
                    .into(),
            );
            return Ok((StatusCode::NO_CONTENT, headers));
        }
    }
    let media = s.data().media2_select(media_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert("upload-offset", media.size.into());
    headers.insert("upload-length", media.size.into());
    Ok((StatusCode::NO_CONTENT, headers))
}

/// Media delete
///
/// Delete unlinked media. Does not work if the media is linked to some other
/// resource.
#[utoipa::path(
    delete,
    path = "/media/{media_id}",
    tags = ["media", "badge.scope.full"],
    params(("media_id", description = "Media id")),
    responses(
        (status = NO_CONTENT, description = "no content"),
        (status = CONFLICT, description = "media is linked to another resource (ie. a message)"),
    )
)]
async fn media_delete(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    if let Some(up) = s.services().media.uploads.get_mut(&media_id) {
        if up.user_id == auth.user.id {
            s.services().media.uploads.remove(&media_id);
        }
        Ok(StatusCode::NO_CONTENT)
    } else {
        let links = s.data().media2_link_select(media_id).await?;
        if links.is_empty() {
            s.data().media2_delete(media_id).await?;
            Ok(StatusCode::NO_CONTENT)
        } else {
            Ok(StatusCode::CONFLICT)
        }
    }
}

/// Media clone
///
/// Create a new unconsumed copy of a piece of media
#[utoipa::path(
    post,
    path = "/media/{media_id}/clone",
    tags = ["media", "badge.scope.full"],
    params(("media_id", description = "Media id")),
    responses(
        (status = OK, description = "success"),
    )
)]
async fn media_clone(
    Path(_media_id): Path<MediaId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<MediaClone>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    Ok(Error::Unimplemented)
}

/// Media search
///
/// Search media. Admins can search all media, everyone else can only search their own media.
#[utoipa::path(
    post,
    path = "/media/search",
    tags = ["media", "badge.scope.full", "badge.admin_only"],
    responses((status = OK, description = "success")),
)]
async fn media_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(_json): Json<MediaSearch>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(common::v1::types::Permission::Admin)?;
    Ok(Error::Unimplemented)
}

/// Media upload direct
///
/// Directly upload a piece of media without doing the whole create/patch/done
/// dance. Only use this for small media.
// TODO: allow async puploads here too
#[utoipa::path(
    post,
    path = "/media/direct",
    tags = ["media", "badge.scope.full"],
    responses((status = ACCEPTED, description = "Processing in background")),
)]
async fn media_upload_direct(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let media_id = MediaId::new();
    let mut data = None;

    while let Some(field) = multipart.next_field().await? {
        match field
            .name()
            .ok_or(Error::BadStatic("field is missing name"))?
        {
            // TODO: parse {filename, alt, process_async, strip_exif}
            "json" => return Err(Error::Unimplemented),
            "file" => {
                data = Some(field.bytes().await?);
            }
            _ => return Err(Error::BadStatic("unknown field")),
        }
    }

    let Some(data) = data else {
        return Err(Error::BadStatic("no data"));
    };

    srv.media
        .create_upload(
            media_id,
            auth.user.id,
            common::v2::types::media::MediaCreate {
                alt: None,
                strip_exif: false,
                source: common::v2::types::media::MediaCreateSource::Upload {
                    filename: "unknown".to_owned(),
                    size: Some(data.len() as u64),
                },
            }
            .into(),
        )
        .await?;

    let mut up = srv.media.uploads.get_mut(&media_id).unwrap();
    if let Err(err) = up.write(&data).await {
        srv.media.uploads.remove(&media_id);
        return Err(err);
    }

    up.temp_writer.flush().await?;

    let (_, up) = s
        .services()
        .media
        .uploads
        .remove(&media_id)
        .expect("it was there a few milliseconds ago");

    let filename = match &up.create.source {
        common::v2::types::media::MediaCreateSource::Upload { filename, .. } => filename.to_owned(),
        common::v2::types::media::MediaCreateSource::Download { .. } => {
            panic!("can only patch upload")
        }
    };

    let media = s
        .services()
        .media
        .process_upload(up, media_id, auth.user.id, &filename)
        .await?;

    Ok((StatusCode::CREATED, Json(media)))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(media_create))
        .routes(routes!(media_patch))
        .routes(routes!(media_get))
        .routes(routes!(media_delete))
        .routes(routes!(media_done))
        .routes(routes!(media_clone))
        .routes(routes!(media_upload_direct))
        .routes(routes!(media_search))
        // TODO: move these to cdn?
        .route(
            "/internal/media-upload/{media_id}",
            routing::patch(media_upload).head(media_check),
        )
}
