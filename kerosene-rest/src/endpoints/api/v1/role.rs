use std::collections::HashSet;

use common::v1::types::{
    AuditLogEntryType, MessageSync, PaginationResponse, Permission, oauth::Scope, util::Diff,
};
use http::StatusCode;
use kerosene_core::types::permission::{CheckPermissions, Permissions2};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::role_create)]
pub async fn create(
    req: Req<routes::role_create::Endpoint>,
) -> Result<routes::role_create::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut auth = ReqAuth::new(&req);
    let role = srv
        .role
        .create(
            body.room_id,
            &mut auth,
            body.role.clone(),
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::role_create::Response { role })
}

#[handler(routes::role_update)]
pub async fn update(
    req: Req<routes::role_update::Endpoint>,
) -> Result<routes::role_update::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let mut data = globals.begin_read().await.cast_internal()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(user.id))
        .await
        .cast_internal()?;
    if room.security.require_sudo {
        identity.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoleManage);

    let start_role = data
        .role_select(body.room_id, body.role_id)
        .await
        .cast_internal()?;

    let mut patch = body.patch.clone();
    if patch.allow.is_some() && patch.deny.is_none() {
        let mut new_deny = start_role.deny.clone();
        new_deny.retain(|p| !patch.allow.as_ref().unwrap().contains(p));
        if new_deny.len() != start_role.deny.len() {
            patch.deny = Some(new_deny);
        }
    } else if patch.deny.is_some() && patch.allow.is_none() {
        let mut new_allow = start_role.allow.clone();
        new_allow.retain(|p| !patch.deny.as_ref().unwrap().contains(p));
        if new_allow.len() != start_role.allow.len() {
            patch.allow = Some(new_allow);
        }
    }

    let new_allow = patch.allow.as_ref().unwrap_or(&start_role.allow);
    let new_deny = patch.deny.as_ref().unwrap_or(&start_role.deny);

    let allow_set: HashSet<_> = new_allow.iter().collect();
    let deny_set: HashSet<_> = new_deny.iter().collect();
    if !allow_set.is_disjoint(&deny_set) {
        return Err(ApiError::from_code(ErrorCode::PermissionConflict).into());
    }

    if !patch.changes(&start_role) {
        return Ok(routes::role_update::Response {
            role: start_role,
            status: StatusCode::NOT_MODIFIED,
        });
    }

    let rank = srv
        .perms
        .get_user_rank(body.room_id, user.id)
        .await
        .cast_internal()?;
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if rank <= start_role.position && room.owner_id != Some(user.id) {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    if let Some(new_allow) = &patch.allow {
        let new_allow_set: HashSet<Permission> = new_allow.iter().cloned().collect();
        let old_allow_set: HashSet<Permission> = start_role.allow.iter().cloned().collect();

        for p in new_allow_set.symmetric_difference(&old_allow_set) {
            perms.needs(*p);
        }
    }
    if let Some(new_deny) = &patch.deny {
        let new_deny_set: HashSet<Permission> = new_deny.iter().cloned().collect();
        let old_deny_set: HashSet<Permission> = start_role.deny.iter().cloned().collect();

        for p in new_deny_set.symmetric_difference(&old_deny_set) {
            perms.needs(*p);
        }
    }
    perms.check()?;

    let mut auth = ReqAuth::new(&req);
    let role = srv
        .role
        .update(body.room_id, body.role_id, &mut auth, patch)
        .await
        .cast_internal()?;

    Ok(routes::role_update::Response {
        role,
        status: StatusCode::OK,
    })
}

#[handler(routes::role_delete)]
pub async fn delete(
    req: Req<routes::role_delete::Endpoint>,
) -> Result<routes::role_delete::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    if body.room_id.into_inner() == body.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }

    let mut data = globals.begin_read().await.cast_internal()?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.security.require_sudo {
        identity.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoleManage);
    perms.check()?;

    let role = data
        .role_select(body.room_id, body.role_id)
        .await
        .cast_internal()?;
    let rank = srv
        .perms
        .get_user_rank(body.room_id, user.id)
        .await
        .cast_internal()?;
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if rank <= role.position && room.owner_id != Some(user.id) {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    if role.member_count == 0 || body.query.fallback_role_id.is_some() {
        let mut auth = ReqAuth::new(&req);
        srv.role
            .delete(body.room_id, body.role_id, &mut auth)
            .await
            .cast_internal()?;

        Ok(routes::role_delete::Response {
            status: StatusCode::NO_CONTENT,
        })
    } else {
        Ok(routes::role_delete::Response {
            status: StatusCode::CONFLICT,
        })
    }
}

#[handler(routes::role_get)]
pub async fn get(req: Req<routes::role_get::Endpoint>) -> Result<routes::role_get::Response> {
    let identity = req.identity();
    identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let role = data
        .role_select(body.room_id, body.role_id)
        .await
        .cast_internal()?;

    Ok(routes::role_get::Response { role })
}

#[handler(routes::role_list)]
pub async fn list(req: Req<routes::role_list::Endpoint>) -> Result<routes::role_list::Response> {
    let identity = req.identity();
    identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?;

    let roles = srv.role.list(body.room_id).await.cast_internal()?;
    let total = roles.len() as u64;

    Ok(routes::role_list::Response {
        roles: PaginationResponse {
            items: roles,
            total,
            has_more: false,
            cursor: None,
        },
    })
}

#[handler(routes::role_member_list)]
pub async fn member_list(
    req: Req<routes::role_member_list::Endpoint>,
) -> Result<routes::role_member_list::Response> {
    let identity = req.identity();
    identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let members = data
        .role_member_list(body.role_id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::role_member_list::Response { members })
}

#[handler(routes::role_member_add)]
pub async fn member_add(
    req: Req<routes::role_member_add::Endpoint>,
) -> Result<routes::role_member_add::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    if body.room_id.into_inner() == body.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }

    let mut data = globals.begin().await.cast_internal()?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.security.require_mfa {
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoleApply);
    perms.check()?;

    let role = data
        .role_select(body.room_id, body.role_id)
        .await
        .cast_internal()?;
    let rank = srv
        .perms
        .get_user_rank(body.room_id, user.id)
        .await
        .cast_internal()?;
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    let self_apply = role.is_self_applicable && body.user_id == user.id;
    if rank <= role.position && room.owner_id != Some(user.id) && !self_apply {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    data.role_member_put(body.room_id, body.user_id, body.role_id)
        .await
        .cast_internal()?;
    let member = data
        .room_member_get(body.room_id, body.user_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    let target_user = srv.users.get(body.user_id, None).await.cast_internal()?;
    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::RoleApply {
                user_id: body.user_id,
                role_id: body.role_id,
            },
        )
        .success();
    srv.perms.invalidate_room(body.user_id, body.room_id).await;
    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomMemberUpdate {
                member: member.clone(),
                user: target_user,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::role_member_add::Response { member })
}

#[handler(routes::role_member_remove)]
pub async fn member_remove(
    req: Req<routes::role_member_remove::Endpoint>,
) -> Result<routes::role_member_remove::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    if body.room_id.into_inner() == body.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }

    let mut data = globals.begin().await.cast_internal()?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.security.require_mfa {
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoleApply);
    perms.check()?;

    let role = data
        .role_select(body.room_id, body.role_id)
        .await
        .cast_internal()?;
    let rank = srv
        .perms
        .get_user_rank(body.room_id, user.id)
        .await
        .cast_internal()?;
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    let self_remove = role.is_self_applicable && body.user_id == user.id;
    if rank <= role.position && room.owner_id != Some(user.id) && !self_remove {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    data.role_member_delete(body.room_id, body.user_id, body.role_id)
        .await
        .cast_internal()?;
    let member = data
        .room_member_get(body.room_id, body.user_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    let target_user = srv.users.get(body.user_id, None).await.cast_internal()?;
    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::RoleUnapply {
                user_id: body.user_id,
                role_id: body.role_id,
            },
        )
        .success();
    srv.perms.invalidate_room(body.user_id, body.room_id).await;
    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomMemberUpdate {
                member: member.clone(),
                user: target_user,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::role_member_remove::Response {})
}

#[handler(routes::role_member_bulk_patch)]
pub async fn member_bulk_patch(
    req: Req<routes::role_member_bulk_patch::Endpoint>,
) -> Result<routes::role_member_bulk_patch::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    if body.room_id.into_inner() == body.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }

    let mut data = globals.begin().await.cast_internal()?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.security.require_mfa {
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoleApply);
    perms.check()?;

    let role = data
        .role_select(body.room_id, body.role_id)
        .await
        .cast_internal()?;
    let rank = srv
        .perms
        .get_user_rank(body.room_id, user.id)
        .await
        .cast_internal()?;
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;

    if rank <= role.position && room.owner_id != Some(user.id) {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    for user_id in &body.patch.apply {
        let target_rank = srv
            .perms
            .get_user_rank(body.room_id, *user_id)
            .await
            .cast_internal()?;
        if rank <= target_rank && room.owner_id != Some(user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRankToManageUser).into());
        }
    }
    for user_id in &body.patch.remove {
        let target_rank = srv
            .perms
            .get_user_rank(body.room_id, *user_id)
            .await
            .cast_internal()?;
        if rank <= target_rank && room.owner_id != Some(user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRankToManageUser).into());
        }
    }

    data.role_member_bulk_edit(
        body.room_id,
        body.role_id,
        &body.patch.apply,
        &body.patch.remove,
    )
    .await
    .cast_internal()?;

    let all_user_ids: Vec<_> = body
        .patch
        .apply
        .iter()
        .chain(body.patch.remove.iter())
        .copied()
        .collect();

    let mut updated = Vec::with_capacity(all_user_ids.len());
    for user_id in all_user_ids {
        let member = data
            .room_member_get(body.room_id, user_id)
            .await
            .cast_internal()?;
        let target_user = srv.users.get(user_id, None).await.cast_internal()?;
        updated.push((user_id, member, target_user));
    }
    data.commit().await.cast_internal()?;

    for (user_id, member, target_user) in updated {
        if body.patch.apply.contains(&user_id) {
            req.audit_log()
                .push(
                    body.room_id,
                    AuditLogEntryType::RoleApply {
                        user_id,
                        role_id: body.role_id,
                    },
                )
                .success();
        }
        if body.patch.remove.contains(&user_id) {
            req.audit_log()
                .push(
                    body.room_id,
                    AuditLogEntryType::RoleUnapply {
                        user_id,
                        role_id: body.role_id,
                    },
                )
                .success();
        }

        srv.perms.invalidate_room(user_id, body.room_id).await;
        globals
            .messaging()
            .broadcast_room(
                body.room_id,
                MessageSync::RoomMemberUpdate {
                    member,
                    user: target_user,
                },
            )
            .await
            .cast_internal()?;
    }

    Ok(routes::role_member_bulk_patch::Response {})
}

#[handler(routes::role_reorder)]
pub async fn reorder(
    req: Req<routes::role_reorder::Endpoint>,
) -> Result<routes::role_reorder::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    body.reorder
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let mut data = globals.begin().await.cast_internal()?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.security.require_sudo {
        identity.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::RoleManage);
    perms.check()?;

    let rank = srv
        .perms
        .get_user_rank(body.room_id, user.id)
        .await
        .cast_internal()?;
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;

    for r in &body.reorder.roles {
        let role = data
            .role_select(body.room_id, r.role_id)
            .await
            .cast_internal()?;
        if rank <= role.position && room.owner_id != Some(user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
        if r.position >= rank && room.owner_id != Some(user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
    }

    data.role_reorder(body.room_id, body.reorder.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::RoleReorder {
                roles: body.reorder.roles.clone(),
            },
        )
        .success();

    srv.perms.invalidate_room_all(body.room_id).await;
    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoleReorder {
                room_id: body.room_id,
                roles: body.reorder.roles.clone(),
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::role_reorder::Response {})
}

export_routes!(
    create,
    update,
    delete,
    get,
    list,
    member_list,
    member_add,
    member_remove,
    member_bulk_patch,
    reorder,
);
