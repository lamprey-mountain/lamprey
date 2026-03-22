use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::room_template::{
    RoomTemplate, RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch,
};
use common::v1::types::Permission;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::types::{PaginationQuery, PaginationResponse};
use crate::{routes2, ServerState};

/// Room template create
#[handler(routes::room_template_create)]
async fn room_template_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_template_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.template.validate()?;

    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.template.room_id)
        .await?;
    perms.ensure(Permission::RoomEdit)?;

    let template = s
        .services()
        .room_templates
        .create(auth.user.id, req.template)
        .await?;

    Ok((StatusCode::CREATED, Json(template)))
}

/// Room template list
#[handler(routes::room_template_list)]
async fn room_template_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_template_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let response = s
        .services()
        .room_templates
        .list(auth.user.id, req.pagination)
        .await?;

    Ok(Json(response))
}

/// Room template get
#[handler(routes::room_template_get)]
async fn room_template_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_template_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let template = s.services().room_templates.get(req.code).await?;
    Ok(Json(template))
}

/// Room template edit
#[handler(routes::room_template_edit)]
async fn room_template_edit(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_template_edit::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.patch.validate()?;

    let template = s.services().room_templates.get(req.code.clone()).await?;
    if template.creator.id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    let updated = s
        .services()
        .room_templates
        .update(req.code, req.patch)
        .await?;
    Ok(Json(updated))
}

/// Room template delete
#[handler(routes::room_template_delete)]
async fn room_template_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_template_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let template = s.services().room_templates.get(req.code.clone()).await?;
    if template.creator.id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    s.services().room_templates.delete(req.code).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room template sync
#[handler(routes::room_template_sync)]
async fn room_template_sync(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_template_sync::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let template = s.services().room_templates.get(req.code.clone()).await?;
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

    let updated = s.services().room_templates.sync(req.code).await?;
    Ok(Json(updated))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(room_template_create))
        .routes(routes2!(room_template_list))
        .routes(routes2!(room_template_get))
        .routes(routes2!(room_template_edit))
        .routes(routes2!(room_template_delete))
        .routes(routes2!(room_template_sync))
}
