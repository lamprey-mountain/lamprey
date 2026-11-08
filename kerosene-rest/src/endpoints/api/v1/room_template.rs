use common::v1::types::{Permission, oauth::Scope};
use lamprey_backend_core::types::permission::{CheckPermissions, Permissions2};

use crate::prelude::*;

#[handler(routes::room_template_create)]
pub async fn create(
    req: Req<routes::room_template_create::Endpoint>,
) -> Result<routes::room_template_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.template
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_room3(Some(user.id), body.template.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoomEdit);
    perms.check()?;

    let template = srv
        .room_templates
        .create(user.id, body.template.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_template_create::Response { template })
}

#[handler(routes::room_template_list)]
pub async fn list(
    req: Req<routes::room_template_list::Endpoint>,
) -> Result<routes::room_template_list::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let templates = srv
        .room_templates
        .list(user.id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_template_list::Response { templates })
}

#[handler(routes::room_template_get)]
pub async fn get(
    req: Req<routes::room_template_get::Endpoint>,
) -> Result<routes::room_template_get::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let template = req
        .services()
        .room_templates
        .get(req.inner().code.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_template_get::Response { template })
}

#[handler(routes::room_template_edit)]
pub async fn edit(
    req: Req<routes::room_template_edit::Endpoint>,
) -> Result<routes::room_template_edit::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let template = srv
        .room_templates
        .get(body.code.clone())
        .await
        .cast_internal()?;
    if template.creator.id != user.id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let template = srv
        .room_templates
        .update(body.code.clone(), body.patch.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_template_edit::Response { template })
}

#[handler(routes::room_template_delete)]
pub async fn delete(
    req: Req<routes::room_template_delete::Endpoint>,
) -> Result<routes::room_template_delete::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let template = srv
        .room_templates
        .get(body.code.clone())
        .await
        .cast_internal()?;
    if template.creator.id != user.id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    srv.room_templates
        .delete(body.code.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_template_delete::Response {})
}

#[handler(routes::room_template_sync)]
pub async fn sync(
    req: Req<routes::room_template_sync::Endpoint>,
) -> Result<routes::room_template_sync::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let template = srv
        .room_templates
        .get(body.code.clone())
        .await
        .cast_internal()?;
    if template.creator.id != user.id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let source_room_id = template
        .source_room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownRoomTemplate))?;

    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_room3(Some(user.id), source_room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoomEdit);
    perms.check()?;

    let template = srv
        .room_templates
        .sync(body.code.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_template_sync::Response { template })
}

export_routes!(create, list, get, edit, delete, sync);
