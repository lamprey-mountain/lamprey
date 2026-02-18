use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntryType, MessageSync, PaginationQuery, PaginationResponse, Permission, Role,
    RoleCreate, RoleId, RoleMemberBulkPatch, RolePatch, RoleReorder, RoomId, RoomMember, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::{DbRoleCreate, RoleDeleteQuery};
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Role create
#[utoipa::path(
    post,
    path = "/room/{room_id}/role",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["role", "badge.perm.RoleManage", "badge.room-sudo", "badge.room-mfa", "badge.audit-log.RoleCreate"],
    responses(
        (status = CREATED, body = Role, description = "success"),
    )
)]
async fn role_create(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoleCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    let srv = s.services();
    let data = s.data();
    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let allow_set: HashSet<_> = json.allow.iter().collect();
    let deny_set: HashSet<_> = json.deny.iter().collect();

    if !allow_set.is_disjoint(&deny_set) {
        return Err(Error::BadRequest(
            "a permission cannot be both allowed and denied".to_string(),
        ));
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleManage)?;

    for p in &json.allow {
        perms.ensure(*p)?;
    }

    let room = srv.rooms.get(room_id, None).await?;
    let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    if rank == 0 && room.owner_id != Some(auth.user.id) {
        // special case: we don't want people with only the base role to be able
        // to create roles, as that role will always be position 1 and won't be
        // able to be edited or applied
        return Err(Error::BadStatic("your rank is too low"));
    }
    let role = data
        .role_create(
            DbRoleCreate {
                id: RoleId::new(),
                room_id,
                name: json.name,
                description: json.description,
                allow: json.allow,
                deny: json.deny,
                is_self_applicable: json.is_self_applicable,
                is_mentionable: json.is_mentionable,
                hoist: json.hoist,
            },
            1,
        )
        .await?;

    let changes = Changes::new()
        .add("name", &role.name)
        .add("description", &role.description)
        .add("allow", &role.allow)
        .add("deny", &role.deny)
        .add("is_self_applicable", &role.is_self_applicable)
        .add("is_mentionable", &role.is_mentionable)
        .add("hoist", &role.hoist)
        .build();

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::RoleCreate { changes })
        .await?;

    let msg = MessageSync::RoleCreate { role: role.clone() };
    s.broadcast_room(room_id, auth.user.id, msg).await?;
    srv.perms.invalidate_user_ranks(room_id);
    Ok((StatusCode::CREATED, Json(role)))
}

/// Role update
#[utoipa::path(
    patch,
    path = "/room/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role", "badge.perm.RoleManage", "badge.room-sudo", "badge.room-mfa", "badge.audit-log.RoleUpdate"],
    responses(
        (status = OK, body = Role, description = "success"),
        (status = NOT_MODIFIED, description = "success"),
    )
)]
async fn role_update(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RolePatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleManage)?;
    let start_role = d.role_select(room_id, role_id).await?;

    let new_allow = json.allow.as_ref().unwrap_or(&start_role.allow);
    let new_deny = json.deny.as_ref().unwrap_or(&start_role.deny);

    let allow_set: HashSet<_> = new_allow.iter().collect();
    let deny_set: HashSet<_> = new_deny.iter().collect();

    if !allow_set.is_disjoint(&deny_set) {
        return Err(Error::BadRequest(
            "a permission cannot be both allowed and denied".to_string(),
        ));
    }

    if !json.changes(&start_role) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    let room = srv.rooms.get(room_id, None).await?;
    if rank <= start_role.position && room.owner_id != Some(auth.user.id) {
        return Err(Error::BadStatic("your rank is too low"));
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
    d.role_update(room_id, role_id, json.clone()).await?;
    let end_role = d.role_select(room_id, role_id).await?;

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

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::RoleUpdate { changes })
        .await?;

    let msg = MessageSync::RoleUpdate {
        role: end_role.clone(),
    };
    if end_role.allow != start_role.allow || end_role.deny != start_role.deny {
        s.services().perms.invalidate_room_all(room_id).await;
    }
    s.broadcast_room(room_id, auth.user.id, msg).await?;
    Ok(Json(end_role).into_response())
}

/// Role delete
#[utoipa::path(
    delete,
    path = "/room/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role", "badge.perm.RoleManage", "badge.room-sudo", "badge.room-mfa", "badge.audit-log.RoleDelete"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn role_delete(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Query(query): Query<RoleDeleteQuery>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    if room_id.into_inner() == role_id.into_inner() {
        return Err(Error::BadStatic("cannot delete the default role"));
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, None).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleManage)?;
    let role = d.role_select(room_id, role_id).await?;
    let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    let room = srv.rooms.get(room_id, None).await?;
    if rank <= role.position && room.owner_id != Some(auth.user.id) {
        return Err(Error::BadStatic("your rank is too low"));
    }
    if role.member_count == 0 || query.force {
        d.role_delete(room_id, role_id).await?;

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::RoleDelete {
            role_id,
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

        let msg = MessageSync::RoleDelete { room_id, role_id };
        srv.perms.invalidate_room_all(room_id).await;
        s.broadcast_room(room_id, auth.user.id, msg).await?;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::CONFLICT)
    }
}

/// Role get
#[utoipa::path(
    get,
    path = "/room/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, body = Role, description = "success"),
    )
)]
async fn role_get(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let _perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    let role = d.role_select(room_id, role_id).await?;
    Ok(Json(role))
}

/// Role list
#[utoipa::path(
    get,
    path = "/room/{room_id}/role",
    params(
        PaginationQuery<RoleId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, body = PaginationResponse<Role>, description = "success"),
    )
)]
async fn role_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<RoleId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let _perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    let res = d.role_list(room_id, paginate).await?;
    Ok(Json(res))
}

/// Role list members
#[utoipa::path(
    get,
    path = "/room/{room_id}/role/{role_id}/member",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, body = PaginationResponse<RoomMember>, description = "success"),
    )
)]
async fn role_member_list(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let _perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    let res = d.role_member_list(role_id, paginate).await?;
    Ok(Json(res))
}

/// Role member apply
#[utoipa::path(
    put,
    path = "/room/{room_id}/role/{role_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
        ("user_id", description = "User id"),
    ),
    tags = ["role", "badge.perm.RoleApply", "badge.room-mfa-opt", "badge.audit-log.RoleApply"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
    )
)]
async fn role_member_add(
    Path((room_id, role_id, target_user_id)): Path<(RoomId, RoleId, UserId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    if room_id.into_inner() == role_id.into_inner() {
        return Err(Error::BadStatic("cannot manually apply the @everyone role"));
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, None).await?;

    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleApply)?;

    let role = d.role_select(room_id, role_id).await?;
    let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    let room = srv.rooms.get(room_id, None).await?;
    let self_apply = role.is_self_applicable && target_user_id == auth.user.id;
    if rank <= role.position && room.owner_id != Some(auth.user.id) && !self_apply {
        return Err(Error::BadStatic("your rank is too low"));
    }

    d.role_member_put(room_id, target_user_id, role_id).await?;
    let member = d.room_member_get(room_id, target_user_id).await?;
    let user = srv.users.get(target_user_id, None).await?;
    let msg = MessageSync::RoomMemberUpdate {
        member: member.clone(),
        user,
    };
    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::RoleApply {
        user_id: target_user_id,
        role_id,
    })
    .await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    s.broadcast_room(room_id, auth.user.id, msg).await?;
    Ok(Json(member))
}

/// Role member remove
#[utoipa::path(
    delete,
    path = "/room/{room_id}/role/{role_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
        ("user_id", description = "User id"),
    ),
    tags = ["role", "badge.perm.RoleApply", "badge.room-mfa-opt", "badge.audit-log.RoleUnapply"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
    )
)]
async fn role_member_remove(
    Path((room_id, role_id, target_user_id)): Path<(RoomId, RoleId, UserId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    if room_id.into_inner() == role_id.into_inner() {
        return Err(Error::BadStatic(
            "cannot manually remove the @everyone role",
        ));
    }
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, None).await?;

    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleApply)?;
    let role = d.role_select(room_id, role_id).await?;
    let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    let room = srv.rooms.get(room_id, None).await?;
    let self_apply = role.is_self_applicable && target_user_id == auth.user.id;
    if rank <= role.position && room.owner_id != Some(auth.user.id) && !self_apply {
        return Err(Error::BadStatic("your rank is too low"));
    }

    d.role_member_delete(room_id, target_user_id, role_id)
        .await?;
    let member = d.room_member_get(room_id, target_user_id).await?;
    let user = srv.users.get(target_user_id, None).await?;
    let msg = MessageSync::RoomMemberUpdate {
        member: member.clone(),
        user,
    };

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::RoleUnapply {
        user_id: target_user_id,
        role_id,
    })
    .await?;

    srv.perms.invalidate_room(target_user_id, room_id).await;
    s.broadcast_room(room_id, auth.user.id, msg).await?;
    Ok(Json(member))
}

/// Role member bulk edit
#[utoipa::path(
    patch,
    path = "/room/{room_id}/role/{role_id}/member",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role", "badge.room-mfa", "badge.audit-log.RoleApply", "badge.audit-log.RoleUnapply"],
    responses(
        (status = NO_CONTENT, body = (), description = "success"),
    )
)]
async fn role_member_bulk_edit(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(body): Json<RoleMemberBulkPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    body.validate()?;

    if room_id.into_inner() == role_id.into_inner() {
        return Err(Error::BadStatic(
            "cannot manually apply or remove the @everyone role",
        ));
    }

    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, None).await?;

    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleApply)?;

    let role = d.role_select(room_id, role_id).await?;
    let auth_user_rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    let room = srv.rooms.get(room_id, None).await?;

    if auth_user_rank <= role.position && room.owner_id != Some(auth.user.id) {
        return Err(Error::BadStatic("your rank is too low"));
    }

    let all_user_ids: Vec<UserId> = body
        .apply
        .iter()
        .chain(body.remove.iter())
        .copied()
        .collect();

    for target_user_id in &body.apply {
        let target_rank = srv.perms.get_user_rank(room_id, *target_user_id).await?;
        if auth_user_rank <= target_rank && room.owner_id != Some(auth.user.id) {
            return Err(Error::BadStatic("your rank is too low to manage this user"));
        }
    }

    for target_user_id in &body.remove {
        let target_rank = srv.perms.get_user_rank(room_id, *target_user_id).await?;
        if auth_user_rank <= target_rank && room.owner_id != Some(auth.user.id) {
            return Err(Error::BadStatic("your rank is too low to manage this user"));
        }
    }

    d.role_member_bulk_edit(room_id, role_id, &body.apply, &body.remove)
        .await?;

    for user_id in all_user_ids {
        let member = d.room_member_get(room_id, user_id).await?;
        let user = srv.users.get(user_id, None).await?;
        let msg = MessageSync::RoomMemberUpdate {
            member: member.clone(),
            user,
        };

        if body.apply.contains(&user_id) {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::RoleApply { user_id, role_id })
                .await?;
        }

        if body.remove.contains(&user_id) {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::RoleUnapply { user_id, role_id })
                .await?;
        }

        srv.perms.invalidate_room(user_id, room_id).await;
        s.broadcast_room(room_id, auth.user.id, msg).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Role reorder
#[utoipa::path(
    patch,
    path = "/room/{room_id}/role",
    params(("room_id", description = "Room id")),
    tags = ["role", "badge.perm.RoleManage", "badge.room-sudo", "badge.room-mfa", "badge.audit-log.RoleReorder"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn role_reorder(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(body): Json<RoleReorder>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    body.validate()?;

    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, None).await?;

    if room.security.require_sudo {
        auth.ensure_sudo()?;
    }
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoleManage)?;

    let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
    let room = srv.rooms.get(room_id, None).await?;

    // FIXME: prevent moving @everyone role from position 0
    for r in &body.roles {
        let role = d.role_select(room_id, r.role_id).await?;
        if rank <= role.position && room.owner_id != Some(auth.user.id) {
            return Err(Error::BadStatic(
                "your rank is too low to reorder one of the roles",
            ));
        }
        if r.position >= rank && room.owner_id != Some(auth.user.id) {
            return Err(Error::BadStatic(
                "you cannot set a role's position to be equal or higher than your own rank",
            ));
        }
    }

    d.role_reorder(room_id, body.clone()).await?;

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::RoleReorder {
        roles: body.roles.clone(),
    })
    .await?;

    s.services().perms.invalidate_room_all(room_id).await;
    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoleReorder {
            room_id,
            roles: body.roles,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(role_create))
        .routes(routes!(role_update))
        .routes(routes!(role_delete))
        .routes(routes!(role_get))
        .routes(routes!(role_list))
        .routes(routes!(role_reorder))
        .routes(routes!(role_member_list))
        .routes(routes!(role_member_add))
        .routes(routes!(role_member_remove))
        .routes(routes!(role_member_bulk_edit))
}
