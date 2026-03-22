use std::collections::HashSet;
use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{AuditLogEntryType, MessageSync, Permission};
use http::StatusCode;
use lamprey_macros::handler;
use validator::Validate;

use crate::routes::util::Auth;
use crate::routes2;
use crate::ServerState;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;

/// Role create
#[handler(routes::role_create)]
async fn role_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.role.validate()?;

    let srv = s.services();
    let role = srv
        .role
        .create(req.room_id, &auth, req.role, req.idempotency_key)
        .await?;

    Ok((StatusCode::CREATED, Json(role)))
}

/// Role update
#[handler(routes::role_update)]
async fn role_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.patch.validate()?;
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoleManage)?;
    let start_role = d.role_select(req.room_id, req.role_id).await?;

    let mut json = req.patch;
    if json.allow.is_some() && json.deny.is_none() {
        let mut new_deny = start_role.deny.clone();
        new_deny.retain(|p| !json.allow.as_ref().unwrap().contains(p));
        if new_deny.len() != start_role.deny.len() {
            json.deny = Some(new_deny);
        }
    } else if json.deny.is_some() && json.allow.is_none() {
        let mut new_allow = start_role.allow.clone();
        new_allow.retain(|p| !json.deny.as_ref().unwrap().contains(p));
        if new_allow.len() != start_role.allow.len() {
            json.allow = Some(new_allow);
        }
    }

    let new_allow = json.allow.as_ref().unwrap_or(&start_role.allow);
    let new_deny = json.deny.as_ref().unwrap_or(&start_role.deny);

    let allow_set: HashSet<_> = new_allow.iter().collect();
    let deny_set: HashSet<_> = new_deny.iter().collect();

    if !allow_set.is_disjoint(&deny_set) {
        return Err(ApiError::from_code(ErrorCode::PermissionConflict).into());
    }

    if !json.changes(&start_role) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
    let room = srv.rooms.get(req.room_id, None).await?;
    if rank <= start_role.position && room.owner_id != Some(auth.user.id) {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    if let Some(new_allow) = &json.allow {
        let new_allow_set: HashSet<Permission> = new_allow.iter().cloned().collect();
        let old_allow_set: HashSet<Permission> = start_role.allow.iter().cloned().collect();

        for p in new_allow_set.symmetric_difference(&old_allow_set) {
            perms.ensure(*p)?;
        }
    }
    if let Some(new_deny) = &json.deny {
        let new_deny_set: HashSet<Permission> = new_deny.iter().cloned().collect();
        let old_deny_set: HashSet<Permission> = start_role.deny.iter().cloned().collect();

        for p in new_deny_set.symmetric_difference(&old_deny_set) {
            perms.ensure(*p)?;
        }
    }
    d.role_update(req.room_id, req.role_id, json.clone())
        .await?;
    let end_role = d.role_select(req.room_id, req.role_id).await?;

    let changes = Changes::new()
        .change("name", &start_role.name, &end_role.name)
        .change(
            "description",
            &start_role.description,
            &end_role.description,
        )
        .change("allow", &start_role.allow, &end_role.allow)
        .change("deny", &start_role.deny, &end_role.deny)
        .change(
            "is_self_applicable",
            &start_role.is_self_applicable,
            &end_role.is_self_applicable,
        )
        .change(
            "is_mentionable",
            &start_role.is_mentionable,
            &end_role.is_mentionable,
        )
        .change("hoist", &start_role.hoist, &end_role.hoist)
        .build();

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::RoleUpdate { changes })
        .await?;

    let msg = MessageSync::RoleUpdate {
        role: end_role.clone(),
    };
    if end_role.allow != start_role.allow || end_role.deny != start_role.deny {
        s.services().perms.invalidate_room_all(req.room_id).await;
    }
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;
    Ok(Json(end_role).into_response())
}

/// Role delete
#[handler(routes::role_delete)]
async fn role_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    if req.room_id.into_inner() == req.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, None).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoleManage)?;
    let role = d.role_select(req.room_id, req.role_id).await?;
    let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
    let room = srv.rooms.get(req.room_id, None).await?;
    if rank <= role.position && room.owner_id != Some(auth.user.id) {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }
    if role.member_count == 0 || req.fallback_role_id.is_some() {
        d.role_delete(req.room_id, req.role_id).await?;

        let al = auth.audit_log(req.room_id);
        al.commit_success(AuditLogEntryType::RoleDelete {
            role_id: req.role_id,
            changes: Changes::new()
                .remove("name", &role.name)
                .remove("description", &role.description)
                .remove("allow", &role.allow)
                .remove("deny", &role.deny)
                .remove("is_self_applicable", &role.is_self_applicable)
                .remove("is_mentionable", &role.is_mentionable)
                .remove("hoist", &role.hoist)
                .remove("member_count", &role.member_count)
                .build(),
        })
        .await?;

        let msg = MessageSync::RoleDelete {
            room_id: req.room_id,
            role_id: req.role_id,
        };
        srv.perms.invalidate_room_all(req.room_id).await;
        s.broadcast_room(req.room_id, auth.user.id, msg).await?;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::CONFLICT)
    }
}

/// Role get
#[handler(routes::role_get)]
async fn role_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let d = s.data();
    let _perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.room_id)
        .await?;
    let role = d.role_select(req.room_id, req.role_id).await?;
    Ok(Json(role))
}

/// Role list
#[handler(routes::role_list)]
async fn role_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let _perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    let roles = srv.role.list(req.room_id).await?;
    Ok(Json(roles))
}

/// Role member list
#[handler(routes::role_member_list)]
async fn role_member_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_member_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let d = s.data();
    let _perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.room_id)
        .await?;
    let res = d.role_member_list(req.role_id, req.pagination).await?;
    Ok(Json(res))
}

/// Role member add
#[handler(routes::role_member_add)]
async fn role_member_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_member_add::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    if req.room_id.into_inner() == req.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, None).await?;

    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoleApply)?;

    let role = d.role_select(req.room_id, req.role_id).await?;
    let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
    let room = srv.rooms.get(req.room_id, None).await?;
    let self_apply = role.is_self_applicable && req.user_id == auth.user.id;
    if rank <= role.position && room.owner_id != Some(auth.user.id) && !self_apply {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    d.role_member_put(req.room_id, req.user_id, req.role_id)
        .await?;
    let member = d.room_member_get(req.room_id, req.user_id).await?;
    let user = srv.users.get(req.user_id, None).await?;
    let msg = MessageSync::RoomMemberUpdate {
        member: member.clone(),
        user,
    };
    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::RoleApply {
        user_id: req.user_id,
        role_id: req.role_id,
    })
    .await?;
    srv.perms.invalidate_room(req.user_id, req.room_id).await;
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;
    Ok(Json(member))
}

/// Role member remove
#[handler(routes::role_member_remove)]
async fn role_member_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_member_remove::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    if req.room_id.into_inner() == req.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, None).await?;

    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoleApply)?;

    let role = d.role_select(req.room_id, req.role_id).await?;
    let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
    let room = srv.rooms.get(req.room_id, None).await?;
    let self_remove = role.is_self_applicable && req.user_id == auth.user.id;
    if rank <= role.position && room.owner_id != Some(auth.user.id) && !self_remove {
        return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
    }

    d.role_member_delete(req.room_id, req.user_id, req.role_id)
        .await?;
    let member = d.room_member_get(req.room_id, req.user_id).await?;
    let user = srv.users.get(req.user_id, None).await?;
    let msg = MessageSync::RoomMemberUpdate {
        member: member.clone(),
        user,
    };
    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::RoleUnapply {
        user_id: req.user_id,
        role_id: req.role_id,
    })
    .await?;
    srv.perms.invalidate_room(req.user_id, req.room_id).await;
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Role member bulk patch
#[handler(routes::role_member_bulk_patch)]
async fn role_member_bulk_patch(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_member_bulk_patch::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    if req.room_id.into_inner() == req.role_id.into_inner() {
        return Err(ApiError::from_code(ErrorCode::CannotModifyDefaultRole).into());
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, None).await?;

    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoleApply)?;

    let role = d.role_select(req.room_id, req.role_id).await?;
    let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
    let room = srv.rooms.get(req.room_id, None).await?;

    // Process apply (add role to users)
    for user_id in &req.patch.apply {
        let self_apply = role.is_self_applicable && *user_id == auth.user.id;
        if rank <= role.position && room.owner_id != Some(auth.user.id) && !self_apply {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }

        d.role_member_put(req.room_id, *user_id, req.role_id)
            .await?;
        let member = d.room_member_get(req.room_id, *user_id).await?;
        let user = srv.users.get(*user_id, None).await?;
        let msg = MessageSync::RoomMemberUpdate {
            member: member.clone(),
            user,
        };
        let al = auth.audit_log(req.room_id);
        al.commit_success(AuditLogEntryType::RoleApply {
            user_id: *user_id,
            role_id: req.role_id,
        })
        .await?;
        srv.perms.invalidate_room(*user_id, req.room_id).await;
        s.broadcast_room(req.room_id, auth.user.id, msg).await?;
    }

    // Process remove (remove role from users)
    for user_id in &req.patch.remove {
        d.role_member_delete(req.room_id, *user_id, req.role_id)
            .await?;
        let member = d.room_member_get(req.room_id, *user_id).await?;
        let user = srv.users.get(*user_id, None).await?;
        let msg = MessageSync::RoomMemberUpdate {
            member: member.clone(),
            user,
        };
        let al = auth.audit_log(req.room_id);
        al.commit_success(AuditLogEntryType::RoleUnapply {
            user_id: *user_id,
            role_id: req.role_id,
        })
        .await?;
        srv.perms.invalidate_room(*user_id, req.room_id).await;
        s.broadcast_room(req.room_id, auth.user.id, msg).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Role reorder
#[handler(routes::role_reorder)]
async fn role_reorder(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::role_reorder::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.reorder.validate()?;

    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, None).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoleManage)?;

    let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
    let room = srv.rooms.get(req.room_id, None).await?;

    for r in &req.reorder.roles {
        let role = d.role_select(req.room_id, r.role_id).await?;
        if rank <= role.position && room.owner_id != Some(auth.user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
        if r.position >= rank && room.owner_id != Some(auth.user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
    }

    d.role_reorder(req.room_id, req.reorder.clone()).await?;

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::RoleReorder {
        roles: req.reorder.roles.clone(),
    })
    .await?;

    s.services().perms.invalidate_room_all(req.room_id).await;
    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::RoleReorder {
            room_id: req.room_id,
            roles: req.reorder.roles,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(role_create))
        .routes(routes2!(role_update))
        .routes(routes2!(role_delete))
        .routes(routes2!(role_get))
        .routes(routes2!(role_list))
        .routes(routes2!(role_reorder))
        .routes(routes2!(role_member_list))
        .routes(routes2!(role_member_add))
        .routes(routes2!(role_member_remove))
        .routes(routes2!(role_member_bulk_patch))
}
