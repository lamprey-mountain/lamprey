use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::room_template::{
    RoomTemplate, RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch,
};
use common::v1::types::Permission;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
    types::{PaginationQuery, PaginationResponse},
    Error, ServerState,
};

use super::util::Auth;

/// Room template create
#[utoipa::path(
    post,
    path = "/room-template",
    tags = ["room_template", "badge.scope.full"],
    request_body = RoomTemplateCreate,
    responses(
        (status = 201, description = "Template created", body = RoomTemplate),
    )
)]
async fn room_template_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomTemplateCreate>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, json.room_id)
        .await?;
    perms.ensure(Permission::RoomEdit)?;

    let template = s
        .services()
        .room_templates
        .create(auth.user.id, json)
        .await?;

    Ok((StatusCode::CREATED, Json(template)))
}

/// Room template list
#[utoipa::path(
    get,
    path = "/room-template",
    tags = ["room_template", "badge.scope.full"],
    params(PaginationQuery<RoomTemplateCode>),
    responses(
        (status = 200, description = "Paginate templates", body = PaginationResponse<RoomTemplate>),
    )
)]
async fn room_template_list(
    Query(q): Query<PaginationQuery<RoomTemplateCode>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let response = s.services().room_templates.list(auth.user.id, q).await?;

    Ok(Json(response))
}

/// Room template get
#[utoipa::path(
    get,
    path = "/room-template/{code}",
    tags = ["room_template", "badge.scope.full"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    responses(
        (status = 200, description = "Get template success", body = RoomTemplate),
    )
)]
async fn room_template_get(
    Path(code): Path<RoomTemplateCode>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let template = s.services().room_templates.get(code).await?;
    Ok(Json(template))
}

/// Room template edit
#[utoipa::path(
    patch,
    path = "/room-template/{code}",
    tags = ["room_template", "badge.scope.full"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    request_body = RoomTemplatePatch,
    responses(
        (status = 200, description = "Edit template success", body = RoomTemplate),
    )
)]
async fn room_template_edit(
    Path(code): Path<RoomTemplateCode>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomTemplatePatch>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    let template = s.services().room_templates.get(code.clone()).await?;
    if template.creator.id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    let updated = s.services().room_templates.update(code, json).await?;
    Ok(Json(updated))
}

/// Room template delete
#[utoipa::path(
    delete,
    path = "/room-template/{code}",
    tags = ["room_template", "badge.scope.full"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    responses(
        (status = 204, description = "Delete template success"),
    )
)]
async fn room_template_delete(
    Path(code): Path<RoomTemplateCode>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let template = s.services().room_templates.get(code.clone()).await?;
    if template.creator.id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    s.services().room_templates.delete(code).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room template sync
#[utoipa::path(
    post,
    path = "/room-template/{code}/sync",
    tags = ["room_template", "badge.scope.full"],
    params(("code" = RoomTemplateCode, Path, description = "Template code")),
    responses(
        (status = 200, description = "Sync template success", body = RoomTemplate),
    )
)]
async fn room_template_sync(
    Path(code): Path<RoomTemplateCode>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let template = s.services().room_templates.get(code.clone()).await?;
    if template.creator.id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    let source_room_id = template
        .source_room_id
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownRoomTemplate,
        )))?;

    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, source_room_id)
        .await?;
    perms.ensure(Permission::RoomEdit)?;

    let updated = s.services().room_templates.sync(code).await?;
    Ok(Json(updated))
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
