use common::v1::types::{Permission, SERVER_ROOM_ID, User, UserId, oauth::Scope};

use crate::prelude::*;

#[handler(routes::harvest_user_get)]
pub async fn user_get(
    req: Req<routes::harvest_user_get::Endpoint>,
) -> Result<routes::harvest_user_get::Response> {
    let srv = req.services();
    let auth_user = authenticate(&req)?;

    let target_user_id = req.inner().user_id.clone().local_unwrap_or(auth_user.id)?;
    if target_user_id != auth_user.id {
        ensure_server_permission(&req.globals(), auth_user.id, Permission::UserManage).await?;
    }

    let harvest = srv.harvest.get_user(target_user_id).await.cast_internal()?;

    Ok(routes::harvest_user_get::Response { harvest })
}

#[handler(routes::harvest_user_create)]
pub async fn user_create(
    req: Req<routes::harvest_user_create::Endpoint>,
) -> Result<routes::harvest_user_create::Response> {
    let srv = req.services();
    let auth_user = authenticate(&req)?;

    let target_user_id = req.inner().user_id.clone().local_unwrap_or(auth_user.id)?;
    if target_user_id != auth_user.id {
        auth_user.ensure_unsuspended()?;
        ensure_server_permission(&req.globals(), auth_user.id, Permission::UserManage).await?;
    }

    srv.harvest
        .create_user(target_user_id, &req.inner().harvest, auth_user.id)
        .await
        .cast_internal()?;

    Ok(routes::harvest_user_create::Response {})
}

#[handler(routes::harvest_get)]
pub async fn get(req: Req<routes::harvest_get::Endpoint>) -> Result<routes::harvest_get::Response> {
    let srv = req.services();
    let auth_user = authenticate(&req)?;

    let harvest = srv
        .harvest
        .get(req.inner().harvest_id)
        .await
        .cast_internal()?;

    if harvest.requester_id != auth_user.id {
        let perms = srv
            .perms
            .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
            .await
            .cast_internal()?;
        if !perms.has(Permission::Admin) {
            // don't leak the existence of other users' harvests
            return Err(ApiError::from_code(ErrorCode::UnknownHarvest).into());
        }
    }

    Ok(routes::harvest_get::Response { harvest })
}

#[handler(routes::harvest_room_get)]
pub async fn room_get(
    req: Req<routes::harvest_room_get::Endpoint>,
) -> Result<routes::harvest_room_get::Response> {
    let srv = req.services();
    let auth_user = authenticate(&req)?;
    ensure_server_permission(&req.globals(), auth_user.id, Permission::RoomManage).await?;

    let harvest = srv
        .harvest
        .get_room(req.inner().room_id)
        .await
        .cast_internal()?;

    Ok(routes::harvest_room_get::Response { harvest })
}

#[handler(routes::harvest_room_create)]
pub async fn room_create(
    req: Req<routes::harvest_room_create::Endpoint>,
) -> Result<routes::harvest_room_create::Response> {
    let srv = req.services();
    let auth_user = authenticate(&req)?;
    auth_user.ensure_unsuspended()?;
    ensure_server_permission(&req.globals(), auth_user.id, Permission::RoomManage).await?;

    srv.harvest
        .create_room(req.inner().room_id, &req.inner().harvest, auth_user.id)
        .await
        .cast_internal()?;

    Ok(routes::harvest_room_create::Response {})
}

/// ensure the request is made by a user that can manage their own harvests
fn authenticate<E: Endpoint>(req: &Req<E>) -> ApiResult<&User> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    // harvests are only for real users
    if user.bot || user.webhook.is_some() || user.puppet.is_some() {
        return Err(ApiError::from_code(ErrorCode::BotsCannotUseThisEndpoint));
    }

    Ok(user)
}

/// require a permission in the server room
async fn ensure_server_permission(
    globals: &Globals,
    user_id: UserId,
    permission: Permission,
) -> Result<()> {
    let mut perms = globals
        .services()
        .perms
        .for_room3(Some(user_id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(permission).check()?;

    Ok(())
}

export_routes!(user_get, user_create, get, room_get, room_create);
