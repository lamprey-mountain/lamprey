use std::collections::HashSet;

use common::v1::types::search::{
    Order, RoomMemberSearchOrderField, RoomMemberSearchRequest, SearchRequest,
};
use common::v1::types::{
    AuditLogEntryType, MessageSync, PaginationResponse, Permission, RoleId, RoomMemberOrigin,
    SERVER_ROOM_ID,
    oauth::Scope,
    util::Changes,
    util::{Diff, Time},
};
use common::v2::types::ApplicationId;
use lamprey_backend_core::types::permission::{CheckPermissions, Permissions2};
use lamprey_backend_services::services::automod::AutomodContext;

use crate::prelude::*;

#[handler(routes::room_member_list)]
pub async fn list(
    req: Req<routes::room_member_list::Endpoint>,
) -> Result<routes::room_member_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    if body.room_id == SERVER_ROOM_ID {
        perms.needs(Permission::ServerOversee);
        identity.ensure_user()?;
    }
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let members = data
        .room_member_list(body.room_id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_member_list::Response { members })
}

#[handler(routes::room_member_get)]
pub async fn get(
    req: Req<routes::room_member_get::Endpoint>,
) -> Result<routes::room_member_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let target_user_id = match &body.user_id {
        common::v1::types::misc::UserIdReq::UserSelf => identity.ensure_user()?.id,
        common::v1::types::misc::UserIdReq::UserId(id) => *id,
        common::v1::types::misc::UserIdReq::RemoteUser(user_id, _) => *user_id,
    };

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let member = data
        .room_member_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;

    Ok(routes::room_member_get::Response { member })
}

#[handler(routes::room_member_add)]
pub async fn add(
    req: Req<routes::room_member_add::Endpoint>,
) -> Result<routes::room_member_add::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().unwrap_or(auth_user.id);

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    if room.security.require_mfa && target_user_id != auth_user.id {
        let totp = globals
            .begin_read()
            .await
            .cast_internal()?
            .auth_totp_get(auth_user.id)
            .await
            .cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let target_user = srv.users.get(target_user_id, None).await.cast_internal()?;

    let mut read = globals.begin_read().await.cast_internal()?;
    let existing = read.room_member_get(body.room_id, target_user_id).await;

    let is_target_other = target_user_id != auth_user.id;
    let is_public_join = target_user_id == auth_user.id && room.public;
    let is_new_join = existing.is_err();
    let origin = if is_public_join {
        if auth_user.registered_at.is_none() {
            return Err(ApiError::from_code(ErrorCode::GuestsCannotJoinPublicRooms).into());
        }

        if let Ok(ban) = read.room_ban_get(body.room_id, target_user_id).await {
            if let Some(expires_at) = ban.expires_at {
                if expires_at > Time::now_utc() {
                    return Err(ApiError::from_code(ErrorCode::YouAreBanned).into());
                }
            } else {
                return Err(ApiError::from_code(ErrorCode::YouAreBanned).into());
            }
        }

        RoomMemberOrigin::PublicJoin
    } else {
        perms.needs(Permission::IntegrationsBridge);

        if !auth_user.bot {
            return Err(ApiError::from_code(ErrorCode::OnlyBotsCanUseThis).into());
        }

        let Some(puppet) = &target_user.puppet else {
            return Err(ApiError::from_code(ErrorCode::CantAddThatUser).into());
        };

        let app_id: ApplicationId = auth_user.id.into_inner().into();
        let app = read.application_get(app_id).await.cast_internal()?;
        if app.bridge.is_none() {
            return Err(ApiError::from_code(ErrorCode::BotIsNotABridge).into());
        }
        if puppet.owner_id != app_id {
            return Err(ApiError::from_code(ErrorCode::NotPuppetOwner).into());
        }

        RoomMemberOrigin::Bridged {
            bridge_id: auth_user.id,
        }
    };

    let mut changes = Changes::new();
    if let Ok(start) = &existing {
        let m = &body.member;

        if m.mute != start.mute {
            changes = changes.change("mute", &start.mute, &m.mute);
            perms.needs(Permission::VoiceMute);
        }
        if m.deaf != start.deaf {
            changes = changes.change("deaf", &start.deaf, &m.deaf);
            perms.needs(Permission::VoiceDeafen);
        }
        if m.override_name != start.override_name {
            changes = changes.change("override_name", &start.override_name, &m.override_name);
            if is_target_other {
                perms.needs(Permission::MemberNicknameManage);
            } else {
                perms.needs(Permission::MemberNickname);
            }
        }
        if m.override_description != start.override_description {
            changes = changes.change(
                "override_description",
                &start.override_description,
                &m.override_description,
            );
        }
        if m.roles != start.roles {
            perms.needs(Permission::RoleApply);
            let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
            let new = HashSet::<RoleId>::from_iter(m.roles.iter().copied());
            let rank = srv
                .perms
                .get_user_rank(body.room_id, target_user_id)
                .await
                .cast_internal()?;

            for role_id in old.difference(&new) {
                let role = read
                    .role_select(body.room_id, *role_id)
                    .await
                    .cast_internal()?;
                if role.position >= rank {
                    return Err(
                        ApiError::from_code(ErrorCode::CannotRemoveRoleAboveYourRole).into(),
                    );
                }
            }
            for role_id in new.difference(&old) {
                let role = read
                    .role_select(body.room_id, *role_id)
                    .await
                    .cast_internal()?;
                if role.position >= rank {
                    return Err(ApiError::from_code(ErrorCode::CannotAddRoleAboveYourRole).into());
                }
            }

            changes = changes.change("roles", &start.roles, &m.roles);
        }
    } else {
        let m = &body.member;

        if m.mute {
            perms.needs(Permission::VoiceMute);
            changes = changes.add("mute", &m.mute);
        }
        if m.deaf {
            perms.needs(Permission::VoiceDeafen);
            changes = changes.add("deaf", &m.deaf);
        }
        if m.override_name.is_some() {
            if is_target_other {
                perms.needs(Permission::MemberNicknameManage);
            } else {
                perms.needs(Permission::MemberNickname);
            }
            changes = changes.add("override_name", &m.override_name);
        }
        if m.override_description.is_some() {
            changes = changes.add("override_description", &m.override_description);
        }
        if !m.roles.is_empty() {
            perms.needs(Permission::RoleApply);
            let rank = srv
                .perms
                .get_user_rank(body.room_id, target_user_id)
                .await
                .cast_internal()?;
            for role_id in &m.roles {
                let role = read
                    .role_select(body.room_id, *role_id)
                    .await
                    .cast_internal()?;
                if role.position >= rank {
                    return Err(ApiError::from_code(ErrorCode::CannotAddRoleAboveYourRole).into());
                }
            }
            changes = changes.add("roles", &m.roles);
        }
    }

    perms.check()?;

    let mut txn = globals.begin().await.cast_internal()?;
    txn.room_member_put(
        body.room_id,
        target_user_id,
        Some(origin),
        body.member.clone(),
    )
    .await
    .cast_internal()?;
    if let Ok(start) = &existing {
        let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
        let new = HashSet::<RoleId>::from_iter(body.member.roles.iter().copied());

        for role_id in old.difference(&new) {
            txn.role_member_delete(body.room_id, target_user_id, *role_id)
                .await
                .cast_internal()?;
        }
        for role_id in new.difference(&old) {
            txn.role_member_put(body.room_id, target_user_id, *role_id)
                .await
                .cast_internal()?;
        }
    } else {
        for role_id in &body.member.roles {
            txn.role_member_put(body.room_id, target_user_id, *role_id)
                .await
                .cast_internal()?;
        }
    }
    txn.commit().await.cast_internal()?;

    srv.perms
        .invalidate_room(target_user_id, body.room_id)
        .await;
    srv.perms.invalidate_is_mutual(target_user_id);

    let mut read = globals.begin_read().await.cast_internal()?;
    let mut res = read
        .room_member_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;

    let automod = srv.automod.load(body.room_id).await.cast_internal()?;
    let automod_ctx = AutomodContext::new(body.room_id, target_user_id);
    let scan = automod.scan(&(&res, &target_user), &automod_ctx).await;
    let has_block_action = scan.should_block();

    if has_block_action || res.quarantined {
        let mut txn = globals.begin().await.cast_internal()?;
        txn.room_member_set_quarantined(body.room_id, target_user_id, has_block_action)
            .await
            .cast_internal()?;
        res = txn
            .room_member_get(body.room_id, target_user_id)
            .await
            .cast_internal()?;
        txn.commit().await.cast_internal()?;
    }

    let changes = changes.build();
    if !changes.is_empty() {
        req.audit_log()
            .push(
                body.room_id,
                AuditLogEntryType::MemberUpdate {
                    room_id: body.room_id,
                    user_id: target_user_id,
                    changes,
                },
            )
            .success();

        let msg = if is_new_join {
            MessageSync::RoomMemberCreate {
                member: res.clone(),
                user: target_user,
            }
        } else {
            MessageSync::RoomMemberUpdate {
                member: res.clone(),
                user: target_user,
            }
        };
        globals
            .messaging()
            .broadcast_room(body.room_id, msg)
            .await
            .cast_internal()?;
    }

    if is_public_join && is_new_join {
        srv.cache
            .send_room_join(body.room_id, target_user_id, Some(res.clone()))
            .await
            .cast_internal()?;
    }

    Ok(routes::room_member_add::Response { member: res })
}

#[handler(routes::room_member_update)]
pub async fn update(
    req: Req<routes::room_member_update::Endpoint>,
) -> Result<routes::room_member_update::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let mut patch = body.patch.clone();
    patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let target_user_id = body.user_id.clone().unwrap_or(auth_user.id);

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let mut data = globals.begin().await.cast_internal()?;
    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room.security.require_mfa {
        let is_self_nickname_change = target_user_id == auth_user.id
            && patch.override_description.is_none()
            && patch.mute.is_none()
            && patch.deaf.is_none()
            && patch.roles.is_none()
            && patch.timeout_until.is_none();

        if !is_self_nickname_change {
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    let start = data
        .room_member_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;
    if !patch.changes(&start) {
        return Ok(routes::room_member_update::Response { member: start });
    }

    if patch.mute.is_some_and(|m| m != start.mute) {
        perms.needs(Permission::VoiceMute);
    }
    if patch.deaf.is_some_and(|m| m != start.deaf) {
        perms.needs(Permission::VoiceDeafen);
    }
    if patch
        .override_name
        .as_ref()
        .is_some_and(|m| m != &start.override_name)
    {
        if target_user_id == auth_user.id {
            perms.needs(Permission::MemberNickname);
        } else {
            perms.needs(Permission::MemberNicknameManage);
        }
    }

    if patch
        .timeout_until
        .as_ref()
        .is_some_and(|val| val != &start.timeout_until)
    {
        perms.needs(Permission::MemberTimeout);
        let rank = srv
            .perms
            .get_user_rank(body.room_id, auth_user.id)
            .await
            .cast_internal()?;
        let other_rank = srv
            .perms
            .get_user_rank(body.room_id, target_user_id)
            .await
            .cast_internal()?;
        let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
        if room.owner_id != Some(auth_user.id) && rank <= other_rank {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
    }

    if let Some(r) = &mut patch.roles {
        r.sort();
        perms.needs(Permission::RoleApply);
        let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
        let new = HashSet::<RoleId>::from_iter(r.iter().copied());
        let rank = srv
            .perms
            .get_user_rank(body.room_id, auth_user.id)
            .await
            .cast_internal()?;

        for role_id in old.difference(&new) {
            let role = data
                .role_select(body.room_id, *role_id)
                .await
                .cast_internal()?;
            if role.position >= rank {
                return Err(ApiError::from_code(ErrorCode::CannotRemoveRoleAboveYourRole).into());
            }
        }
        for role_id in new.difference(&old) {
            let role = data
                .role_select(body.room_id, *role_id)
                .await
                .cast_internal()?;
            if role.position >= rank {
                return Err(ApiError::from_code(ErrorCode::CannotAddRoleAboveYourRole).into());
            }
        }

        for role_id in old.difference(&new) {
            data.role_member_delete(body.room_id, target_user_id, *role_id)
                .await
                .cast_internal()?;
        }
        for role_id in new.difference(&old) {
            data.role_member_put(body.room_id, target_user_id, *role_id)
                .await
                .cast_internal()?;
        }
    }

    perms.check()?;

    data.room_member_patch(body.room_id, target_user_id, patch.clone())
        .await
        .cast_internal()?;
    srv.perms
        .invalidate_room(target_user_id, body.room_id)
        .await;

    let mut res = data
        .room_member_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;

    let automod = srv.automod.load(body.room_id).await.cast_internal()?;
    let automod_ctx = AutomodContext::new(room.id, auth_user.id);
    let scan = automod.scan(&(&res, auth_user), &automod_ctx).await;
    let has_block_action = scan.should_block();

    if has_block_action || res.quarantined {
        data.room_member_set_quarantined(body.room_id, target_user_id, has_block_action)
            .await
            .cast_internal()?;
        res = data
            .room_member_get(body.room_id, target_user_id)
            .await
            .cast_internal()?;
    }

    let changes = Changes::new()
        .change("override_name", &start.override_name, &res.override_name)
        .change(
            "override_description",
            &start.override_description,
            &res.override_description,
        )
        .change("mute", &start.mute, &res.mute)
        .change("deaf", &start.deaf, &res.deaf)
        .change("roles", &start.roles, &res.roles)
        .change("timeout_until", &start.timeout_until, &res.timeout_until)
        .build();

    data.commit().await.cast_internal()?;

    if patch
        .timeout_until
        .as_ref()
        .is_some_and(|val| val != &start.timeout_until)
    {
        srv.perms
            .update_timeout_task(target_user_id, body.room_id, patch.timeout_until.flatten())
            .await;
    }

    if !changes.is_empty() {
        req.audit_log()
            .push(
                body.room_id,
                AuditLogEntryType::MemberUpdate {
                    room_id: body.room_id,
                    user_id: target_user_id,
                    changes,
                },
            )
            .success();
    }

    let user = srv.users.get(res.user_id, None).await.cast_internal()?;
    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomMemberUpdate {
                member: res.clone(),
                user,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_member_update::Response { member: res })
}

#[handler(routes::room_member_delete)]
pub async fn delete(
    req: Req<routes::room_member_delete::Endpoint>,
) -> Result<routes::room_member_delete::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().unwrap_or(auth_user.id);

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room.security.require_mfa && target_user_id != auth_user.id {
        let totp = globals
            .begin_read()
            .await
            .cast_internal()?
            .auth_totp_get(auth_user.id)
            .await
            .cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if target_user_id != auth_user.id {
        perms.needs(Permission::MemberKick);
    }
    perms.check()?;

    if body.room_id == SERVER_ROOM_ID {
        return Err(ApiError::from_code(ErrorCode::CannotKickFromServerRoom).into());
    }
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.owner_id == Some(target_user_id) {
        return Err(ApiError::from_code(ErrorCode::RoomOwnerCannotLeave).into());
    }
    if auth_user.id != target_user_id && room.owner_id != Some(auth_user.id) {
        let rank = srv
            .perms
            .get_user_rank(body.room_id, auth_user.id)
            .await
            .cast_internal()?;
        let other_rank = srv
            .perms
            .get_user_rank(body.room_id, target_user_id)
            .await
            .cast_internal()?;
        if rank <= other_rank {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.room_member_leave(body.room_id, target_user_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms
        .invalidate_room(target_user_id, body.room_id)
        .await;
    srv.perms.invalidate_is_mutual(target_user_id);

    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::MemberKick {
                room_id: body.room_id,
                user_id: target_user_id,
            },
        )
        .success();

    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomMemberDelete {
                room_id: body.room_id,
                user_id: target_user_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_member_delete::Response {})
}

#[handler(routes::room_member_search)]
pub async fn search(
    req: Req<routes::room_member_search::Endpoint>,
) -> Result<routes::room_member_search::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if body.room_id == SERVER_ROOM_ID {
        perms.needs(Permission::ServerOversee);
    }
    perms.check()?;

    let limit = body.search.limit.unwrap_or(10).min(100);
    let search = RoomMemberSearchRequest {
        inner: SearchRequest {
            query: Some(body.search.query.clone()),
            sort_order: Order::Descending,
            limit,
            offset: 0,
        },
        sort_field: RoomMemberSearchOrderField::Joined,
    };
    let results = srv
        .search
        .search_room_members(body.room_id, search)
        .await
        .cast_internal()?;

    Ok(routes::room_member_search::Response { results })
}

#[handler(routes::room_member_search_advanced)]
pub async fn search_advanced(
    req: Req<routes::room_member_search_advanced::Endpoint>,
) -> Result<routes::room_member_search_advanced::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.search
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if body.room_id == SERVER_ROOM_ID {
        perms.needs(Permission::ServerOversee);
    }
    perms.check()?;

    let results = srv
        .search
        .search_room_members(body.room_id, body.search.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_member_search_advanced::Response { results })
}

#[handler(routes::room_prune_begin)]
pub async fn prune_begin(
    req: Req<routes::room_prune_begin::Endpoint>,
) -> Result<routes::room_prune_begin::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.prune
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room.security.require_mfa {
        let totp = req
            .globals()
            .begin_read()
            .await
            .cast_internal()?
            .auth_totp_get(auth_user.id)
            .await
            .cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::MemberKick);
    perms.needs(Permission::RoomEdit);
    perms.check()?;

    Err(Error::Unimplemented)
}

#[handler(routes::room_ban_create)]
pub async fn ban_create(
    req: Req<routes::room_ban_create::Endpoint>,
) -> Result<routes::room_ban_create::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().unwrap_or(auth_user.id);

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room.security.require_mfa {
        let totp = globals
            .begin_read()
            .await
            .cast_internal()?
            .auth_totp_get(auth_user.id)
            .await
            .cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::MemberBan);
    perms.check()?;
    if body.room_id == SERVER_ROOM_ID {
        return Err(ApiError::from_code(ErrorCode::CannotKickFromServerRoom).into());
    }

    let mut data = globals.begin().await.cast_internal()?;
    if data
        .room_member_get(body.room_id, target_user_id)
        .await
        .is_ok()
    {
        let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
        if room.owner_id != Some(auth_user.id) {
            let rank = srv
                .perms
                .get_user_rank(body.room_id, auth_user.id)
                .await
                .cast_internal()?;
            let other_rank = srv
                .perms
                .get_user_rank(body.room_id, target_user_id)
                .await
                .cast_internal()?;
            if rank <= other_rank {
                return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
            }
        }
        if room.owner_id == Some(target_user_id) {
            return Err(ApiError::from_code(ErrorCode::CannotBanRoomOwner).into());
        }
    }

    let reason = req.headers().reason.clone();
    data.room_ban_create(body.room_id, target_user_id, reason, body.ban.expires_at)
        .await
        .cast_internal()?;
    let ban = data
        .room_ban_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;
    data.room_member_leave(body.room_id, target_user_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms
        .invalidate_room(target_user_id, body.room_id)
        .await;
    srv.perms.invalidate_is_mutual(target_user_id);

    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::MemberBan {
                room_id: body.room_id,
                user_id: target_user_id,
            },
        )
        .success();

    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomMemberDelete {
                room_id: body.room_id,
                user_id: target_user_id,
            },
        )
        .await
        .cast_internal()?;
    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::BanCreate {
                room_id: body.room_id,
                ban,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_ban_create::Response {})
}

#[handler(routes::room_ban_bulk_create)]
pub async fn ban_bulk_create(
    req: Req<routes::room_ban_bulk_create::Endpoint>,
) -> Result<routes::room_ban_bulk_create::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    body.ban
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room.security.require_mfa {
        let totp = globals
            .begin_read()
            .await
            .cast_internal()?
            .auth_totp_get(auth_user.id)
            .await
            .cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    srv.perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;
    if body.room_id == SERVER_ROOM_ID {
        return Err(ApiError::from_code(ErrorCode::CannotKickFromServerRoom).into());
    }

    let mut data = globals.begin().await.cast_internal()?;
    let rank = srv
        .perms
        .get_user_rank(body.room_id, auth_user.id)
        .await
        .cast_internal()?;

    for &target_user_id in &body.ban.target_ids {
        if data
            .room_member_get(body.room_id, target_user_id)
            .await
            .is_ok()
            && room.owner_id != Some(auth_user.id)
        {
            let other_rank = srv
                .perms
                .get_user_rank(body.room_id, target_user_id)
                .await
                .cast_internal()?;
            if rank <= other_rank {
                return Err(ApiError::from_code(ErrorCode::InsufficientRankToManageUser).into());
            }
        }
    }

    let reason = req.headers().reason.clone();
    data.room_ban_create_bulk(
        body.room_id,
        &body.ban.target_ids,
        reason,
        body.ban.expires_at,
    )
    .await
    .cast_internal()?;

    for &target_user_id in &body.ban.target_ids {
        data.room_member_leave(body.room_id, target_user_id)
            .await
            .cast_internal()?;
    }
    data.commit().await.cast_internal()?;

    for &target_user_id in &body.ban.target_ids {
        srv.perms
            .invalidate_room(target_user_id, body.room_id)
            .await;
        srv.perms.invalidate_is_mutual(target_user_id);

        req.audit_log()
            .push(
                body.room_id,
                AuditLogEntryType::MemberBan {
                    room_id: body.room_id,
                    user_id: target_user_id,
                },
            )
            .success();

        globals
            .messaging()
            .broadcast_room(
                body.room_id,
                MessageSync::RoomMemberDelete {
                    room_id: body.room_id,
                    user_id: target_user_id,
                },
            )
            .await
            .cast_internal()?;
    }

    Ok(routes::room_ban_bulk_create::Response {})
}

#[handler(routes::room_ban_delete)]
pub async fn ban_delete(
    req: Req<routes::room_ban_delete::Endpoint>,
) -> Result<routes::room_ban_delete::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().unwrap_or(auth_user.id);

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room.security.require_mfa {
        let totp = globals
            .begin_read()
            .await
            .cast_internal()?
            .auth_totp_get(auth_user.id)
            .await
            .cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    srv.perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.room_ban_delete(body.room_id, target_user_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms
        .invalidate_room(target_user_id, body.room_id)
        .await;
    srv.perms.invalidate_is_mutual(target_user_id);

    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::MemberUnban {
                room_id: body.room_id,
                user_id: target_user_id,
            },
        )
        .success();

    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::BanDelete {
                room_id: body.room_id,
                user_id: target_user_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_ban_delete::Response {})
}

#[handler(routes::room_ban_get)]
pub async fn ban_get(
    req: Req<routes::room_ban_get::Endpoint>,
) -> Result<routes::room_ban_get::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().unwrap_or(auth_user.id);

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if !perms.has_any(&[Permission::MemberBan, Permission::AuditLogView]) {
        perms.needs(Permission::MemberBan);
    }
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let ban = data
        .room_ban_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;

    Ok(routes::room_ban_get::Response { ban })
}

#[handler(routes::room_ban_list)]
pub async fn ban_list(
    req: Req<routes::room_ban_list::Endpoint>,
) -> Result<routes::room_ban_list::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if !perms.has_any(&[Permission::MemberBan, Permission::AuditLogView]) {
        perms.needs(Permission::MemberBan);
    }
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let res = data
        .room_ban_list(body.room_id, body.pagination.clone())
        .await
        .cast_internal()?;
    let cursor = res.items.last().map(|i| i.user_id.to_string());

    Ok(routes::room_ban_list::Response {
        bans: PaginationResponse {
            items: res.items,
            has_more: res.has_more,
            total: res.total,
            cursor,
        },
    })
}

#[handler(routes::room_ban_search)]
pub async fn ban_search(
    req: Req<routes::room_ban_search::Endpoint>,
) -> Result<routes::room_ban_search::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_members_manageable()?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if !perms.has_any(&[Permission::MemberBan, Permission::AuditLogView]) {
        perms.needs(Permission::MemberBan);
    }
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let res = data
        .room_ban_search(
            body.room_id,
            body.search.query.clone(),
            body.pagination.clone(),
        )
        .await
        .cast_internal()?;
    let cursor = res.items.last().map(|i| i.user_id.to_string());

    Ok(routes::room_ban_search::Response {
        bans: PaginationResponse {
            items: res.items,
            has_more: res.has_more,
            total: res.total,
            cursor,
        },
    })
}

export_routes!(
    list,
    get,
    add,
    update,
    delete,
    search,
    search_advanced,
    prune_begin,
    ban_create,
    ban_bulk_create,
    ban_delete,
    ban_get,
    ban_list,
    ban_search,
);
