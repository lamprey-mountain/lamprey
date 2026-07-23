use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing,
};
use common::v1::routes;
use common::v2::types::media::{MediaCreateSource, MediaCreated};
use common::{
    v1::types::error::{ApiError, ErrorCode},
    v1::types::{Permission, SERVER_ROOM_ID, application::Scope},
};
use futures_util::StreamExt;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::ServerState;
use crate::{
    error::{Error, Result},
    routes2,
    services::search::SearchMediaVisibility,
};
use common::v1::types::MediaId;
use kerosene_services::services::media::Import;

use super::util::Auth;
use lamprey_backend_core::types::permission::{CheckPermissions, Permissions2};

#[handler(routes::media_create)]
async fn media_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::media_create::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.body.validate()?;

    let srv = s.services();
    let json = req.body;
    match &json.source {
        MediaCreateSource::Upload { size, .. } => {
            if *size > Some(s.config.media.max_size) {
                return Err(Error::TooBig);
            }

            let media_id = MediaId::new();
            let import = Import::new_with_id(media_id, auth.user.id).merge(json.clone());
            srv.media.import_from_upload(import).await?;
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
        MediaCreateSource::Download {
            size, source_url, ..
        } => {
            if size.is_some_and(|sz| sz > s.config.media.max_size) {
                return Err(Error::TooBig);
            }

            let media_id = MediaId::new();
            let import = Import::new_with_id(media_id, auth.user.id).merge(json.clone());
            srv.media.import_from_url(import, source_url).await?;
            let mut headers = HeaderMap::new();
            if let Some(sz) = size {
                headers.insert("content-length", (*sz).into());
            }
            let res = MediaCreated {
                media_id,
                upload_url: None,
            };
            Ok((StatusCode::CREATED, headers, Json(res)))
        }
    }
}

#[handler(routes::media_patch)]
async fn media_patch(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::media_patch::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.body.validate()?;

    let item = s
        .services()
        .media
        .patch(auth.user.id, req.media_id, req.body)
        .await?;

    Ok(Json(item.media()))
}

#[handler(routes::media_done)]
async fn media_done(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::media_done::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let mut item = srv.media.upload_done(req.media_id).await?;
    let media = item.media();

    // FIXME: get FINAL uploaded size, not the size the user provided
    let mut headers = HeaderMap::new();
    headers.insert("upload-offset", media.size.into());
    headers.insert("upload-length", media.size.into());

    if req.body.process_async {
        Ok((StatusCode::ACCEPTED, headers, Json(None)))
    } else {
        let media = item.ready().await;
        Ok((StatusCode::OK, headers, Json(Some(media))))
    }
}

#[handler(routes::media_get)]
async fn media_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::media_get::Request,
) -> Result<impl IntoResponse> {
    let item = s.services().media.get(req.media_id).await?;
    let media = item.media();

    if media.deleted_at.is_some() {
        let perms = s
            .services()
            .perms
            .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
            .await?;
        if !perms.has(Permission::Admin) {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownMedia,
            )));
        }
    }

    Ok(Json(media))
}

#[handler(routes::media_delete)]
async fn media_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::media_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    s.services()
        .media
        .delete(auth.user.id, req.media_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::media_clone)]
async fn media_clone(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::media_clone::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    // TODO: implement srv.media.clone(MediaId)
    Ok(Error::Unimplemented)
}

/// Media upload
///
/// Always returns immediately, but will automatically begin processing media in
/// the background.
async fn media_upload(
    Path(media_id): Path<MediaId>,
    _auth: Auth,
    State(s): State<Arc<ServerState>>,
    headers_req: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse> {
    // TODO: check auth
    let srv = s.services();

    let current_off: u64 = headers_req
        .get("upload-offset")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;
    let _part_length: u64 = headers_req
        .get("content-length")
        .ok_or(Error::BadHeader)?
        .to_str()?
        .parse()?;

    // TODO: ensure body length == part_length? (minor issue)

    {
        let mut up =
            srv.media
                .upload_get(media_id)
                .await
                .ok_or(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownMedia,
                )))?;
        // TODO: return these for headers_res
        up.offset();
        up.expected_size();

        let current_size = up.offset();
        if current_size != current_off {
            return Err(Error::CantOverwrite);
        }

        let mut stream = body.into_data_stream();
        while let Some(chunk) = stream.next().await {
            up.write(&chunk?).await?;
        }

        if !up.expects_more() {
            // drop the lock before calling upload_done
            drop(up);
            srv.media.upload_done(media_id).await?;
        }
    }

    let item = srv.media.get(media_id).await?;
    let media = item.media();

    let mut headers_res = HeaderMap::new();
    headers_res.insert("upload-offset", media.size.into()); // FIXME: should be up.offset()
    headers_res.insert("upload-length", media.size.into()); // FIXME: should be up.expected_size()
    Ok((StatusCode::NO_CONTENT, headers_res))
}

/// Media check
///
/// Get headers useful for resuming an upload
async fn media_check(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let up = srv
        .media
        .upload_get(media_id)
        .await
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownMedia,
        )))?;

    if up.user_id() == auth.user.id {
        let mut headers = HeaderMap::new();
        headers.insert("upload-offset", up.offset().into());
        headers.insert("upload-length", up.expected_size().into());
        Ok((StatusCode::NO_CONTENT, headers))
    } else {
        Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownMedia,
        )))
    }
}

/// Media search
///
/// Search media. Admins can search all media, everyone else can only search their own media.
#[handler(routes::media_search)]
async fn media_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::media_search::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?;
    perms.needs(Permission::Admin);
    perms.check()?;

    let results = srv
        .search
        .search_media(SearchMediaVisibility::Everything, req.body)
        .await?;

    Ok(Json(results))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(media_create))
        .routes(routes2!(media_patch))
        .routes(routes2!(media_get))
        .routes(routes2!(media_delete))
        .routes(routes2!(media_done))
        .routes(routes2!(media_clone))
        .routes(routes2!(media_search))
        // TODO: move these to cdn?
        .route(
            "/internal/media-upload/{media_id}",
            routing::patch(media_upload).head(media_check),
        )
}
