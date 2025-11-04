use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::room_template::{
    RoomTemplate, RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::{Error, Result},
    types::{PaginationQuery, PaginationResponse},
    ServerState,
};

use super::util::Auth;

/// Room template create (TODO)
///
/// create a new reusable room template from an existing room
#[utoipa::path(
    post,
    path = "/room-template",
    tags = ["room-template"],
    request_body = RoomTemplateCreate,
    responses(
        (status = 201, description = "Template created", body = RoomTemplate),
    )
)]
async fn room_template_create(
    Auth(auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(json): Json<RoomTemplateCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;

    Ok(Error::Unimplemented)
}

/// Room template list (TODO)
///
/// list room templates you have created
#[utoipa::path(
    get,
    path = "/room-template",
    tags = ["room-template"],
    params(PaginationQuery<RoomTemplateCode>),
    responses(
        (status = 200, description = "Paginate templates", body = PaginationResponse<RoomTemplate>),
    )
)]
async fn room_template_list(
    Query(_q): Query<PaginationQuery<RoomTemplateCode>>,
    Auth(user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room template get (TODO)
#[utoipa::path(
    get,
    path = "/room-template/{code}",
    tags = ["room-template"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    responses(
        (status = 200, description = "Get template success", body = RoomTemplate),
    )
)]
async fn room_template_get(
    Path(_code): Path<RoomTemplateCode>,
    Auth(user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room template edit (TODO)
#[utoipa::path(
    patch,
    path = "/room-template/{code}",
    tags = ["room-template"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    request_body = RoomTemplatePatch,
    responses(
        (status = 200, description = "Edit template success", body = RoomTemplate),
    )
)]
async fn room_template_edit(
    Path(_code): Path<RoomTemplateCode>,
    Auth(auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(json): Json<RoomTemplatePatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;

    Ok(Error::Unimplemented)
}

/// Room template delete (TODO)
#[utoipa::path(
    delete,
    path = "/room-template/{code}",
    tags = ["room-template"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    responses(
        (status = 204, description = "Delete template success"),
    )
)]
async fn room_template_delete(
    Path(_code): Path<RoomTemplateCode>,
    Auth(auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room template sync (TODO)
#[utoipa::path(
    post,
    path = "/room-template/{code}/sync",
    tags = ["room-template"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    responses(
        (status = 200, description = "Sync template success", body = RoomTemplate),
    )
)]
async fn room_template_sync(
    Path(_code): Path<RoomTemplateCode>,
    Auth(auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_template_create))
        .routes(routes!(room_template_list))
        .routes(routes!(room_template_get))
        .routes(routes!(room_template_edit))
        .routes(routes!(room_template_delete))
        .routes(routes!(room_template_sync))
}
