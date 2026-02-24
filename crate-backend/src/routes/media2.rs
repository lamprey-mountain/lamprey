#![allow(unused)] // TEMP

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::HeaderMap,
    response::IntoResponse,
    routing, Json,
};
use common::{
    v1::types::MediaId,
    v2::types::media::{
        Media, MediaClone, MediaCreate, MediaCreated, MediaDoneParams, MediaPatch, MediaSearch,
    },
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    ServerState,
};

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
    Ok(Error::Unimplemented)
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
    Ok(Error::Unimplemented)
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
    Json(json): Json<MediaDoneParams>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Media upload
// TODO: add utoipa attr
async fn media_upload(
    Path(media_id): Path<MediaId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse> {
    // TODO:
    Ok(Error::Unimplemented)
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
    Path((media_id,)): Path<(MediaId,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
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
    Ok(Error::Unimplemented)
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
    Ok(Error::Unimplemented)
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
    Ok(Error::Unimplemented)
}

/// Media upload direct
///
/// Directly upload a piece of media without doing the whole create/patch/done
/// dance. Only use this for small media.
#[utoipa::path(
    post,
    path = "/media/direct",
    tags = ["media", "badge.scope.full"],
    responses((status = CREATED, description = "success")),
)]
async fn media_upload_direct(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
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
