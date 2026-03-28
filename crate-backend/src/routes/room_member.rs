use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::automod::AutomodAction;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::util::{Diff, Time};
use common::v1::types::{
    AuditLogEntryType, MessageSync, PaginationResponse, Permission, RoomMemberSearchResponse,
    UserId,
};
use common::v1::types::{RoleId, RoomMemberOrigin, SERVER_ROOM_ID};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::routes::util::AuthRelaxed2;
use crate::{routes2, types::UserIdReq, ServerState};
use lamprey_backend_core::types::permission::{CheckPermissions, Permissions2};

use super::util::Auth;
use crate::error::{Error, Result};

/// Room member list
#[handler(routes::room_member_list)]
async fn room_member_list(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let d = s.data();
    let srv = s.services();

    let user_id = auth.user.as_ref().map(|u| u.id);

    let mut perms = srv
        .perms
        .for_room3(user_id, req.room_id)
        .await?
        .ensure_view()?;

    // Extra permission check to prevent returning the entire list of registered users
    // For SERVER_ROOM_ID, require ServerOversee
    if req.room_id == SERVER_ROOM_ID {
        perms.needs(Permission::ServerOversee);
        let _user = auth.ensure_has_user()?;
    }
    perms.check()?;

    let res = d.room_member_list(req.room_id, req.pagination).await?;
    Ok(Json(res))
}

/// Room member get
#[handler(routes::room_member_get)]
async fn room_member_get(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;

    let user_id = auth.user.as_ref().map(|u| u.id);

    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => {
            // Self requires authentication
            let user = auth.ensure_has_user()?;
            user.id
        }
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    srv.perms
        .for_room3(user_id, req.room_id)
        .await?
        .ensure_view()?;
    let res = d.room_member_get(req.room_id, target_user_id).await?;
    Ok(Json(res))
}

// FIXME: only return 304 not modified if an etag is sent
/// Room member add
///
/// - Bots can add puppet users via MemberBridge permission
/// - Users can join public rooms by specifying themselves as the target
/// - Only registered users (not guests) can join public rooms
#[handler(routes::room_member_add)]
async fn room_member_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_add::Request,
) -> Result<impl IntoResponse> {
    let mut req = req;
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let al = auth.audit_log(req.room_id);
    let srv = s.services();
    let data = s.data();
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;

    // allow self joins
    if room.security.require_mfa && target_user_id != auth.user.id {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    // handle joining public rooms
    if target_user_id == auth.user.id {
        let room = srv.rooms.get(req.room_id, None).await?;
        if room.public {
            if auth.user.registered_at.is_none() {
                return Err(ApiError::from_code(ErrorCode::GuestsCannotJoinPublicRooms).into());
            }

            if let Ok(ban) = s.data().room_ban_get(req.room_id, target_user_id).await {
                if let Some(expires_at) = ban.expires_at {
                    if expires_at > Time::now_utc() {
                        return Err(ApiError::from_code(ErrorCode::YouAreBanned).into());
                    }
                } else {
                    return Err(ApiError::from_code(ErrorCode::YouAreBanned).into());
                }
            }

            let d = s.data();
            let existing = d.room_member_get(req.room_id, target_user_id).await;
            let mut perms = if existing.is_ok() {
                // User already exists, get their actual permissions
                s.services()
                    .perms
                    .for_room3(Some(auth.user.id), req.room_id)
                    .await?
            } else {
                // User doesn't exist yet, get default room permissions
                // Use for_room3 to get the new system type
                s.services().perms.default_for_room3(req.room_id).await?
            };
            let mut perms: Permissions2<CheckPermissions> = perms.ensure_view()?;

            if let Ok(start) = &existing {
                // already exists
                if req.member.mute.is_some_and(|m| m != start.mute) {
                    perms.needs(Permission::VoiceMute);
                }
                if req.member.deaf.is_some_and(|m| m != start.deaf) {
                    perms.needs(Permission::VoiceDeafen);
                }
                if req.member.override_name.is_some()
                    && req.member.override_name != start.override_name
                {
                    perms.needs(Permission::MemberNickname);
                }
                if let Some(r) = &mut req.member.roles {
                    // TODO: let users add self applicable roles to themselves
                    // TODO: also handle if @everyone has RoleApply permissions
                    if !r.is_empty() {
                        return Err(ApiError::from_code(ErrorCode::CannotAddRolesToYourself).into());
                    }
                }
            } else {
                // joining for the first time
                if req.member.mute == Some(true) {
                    perms.needs(Permission::VoiceMute);
                }
                if req.member.deaf == Some(true) {
                    perms.needs(Permission::VoiceDeafen);
                }
                if req.member.override_name.is_some() {
                    perms.needs(Permission::MemberNickname);
                }
                if let Some(r) = &mut req.member.roles {
                    // TODO: let users add self applicable roles to themselves
                    // TODO: also handle if @everyone has RoleApply permissions
                    if !r.is_empty() {
                        return Err(ApiError::from_code(ErrorCode::CannotAddRolesToYourself).into());
                    }
                }
            }

            let origin = RoomMemberOrigin::PublicJoin;
            d.room_member_put(
                req.room_id,
                target_user_id,
                Some(origin),
                req.member.clone(),
            )
            .await?;

            s.services()
                .perms
                .invalidate_room(target_user_id, req.room_id)
                .await;
            s.services().perms.invalidate_is_mutual(target_user_id);
            let mut res = d.room_member_get(req.room_id, target_user_id).await?;

            let is_new_join = existing.is_err();

            // handle role updates if any
            if let Some(r) = req.member.roles {
                if let Ok(ref existing) = existing {
                    let old = HashSet::<RoleId>::from_iter(existing.roles.iter().copied());
                    let new = HashSet::<RoleId>::from_iter(r.iter().copied());
                    // removed roles
                    for role_id in old.difference(&new) {
                        d.role_member_delete(req.room_id, target_user_id, *role_id)
                            .await?;
                    }

                    // added roles
                    for role_id in new.difference(&old) {
                        d.role_member_put(req.room_id, target_user_id, *role_id)
                            .await?;
                    }
                } else {
                    for role_id in r {
                        d.role_member_put(req.room_id, target_user_id, role_id)
                            .await?;
                    }
                }
            }

            // scan member with automod
            let automod = srv.automod.load(req.room_id).await?;
            let scan = automod.scan_member(&res, &auth.user);

            let has_block_action = scan
                .actions()
                .iter()
                .any(|action| matches!(action, AutomodAction::Block { .. }));

            if has_block_action {
                d.room_member_set_quarantined(req.room_id, target_user_id, true)
                    .await?;
            } else if res.quarantined {
                d.room_member_set_quarantined(req.room_id, target_user_id, false)
                    .await?;
            }

            if has_block_action || (!has_block_action && res.quarantined) {
                res = d.room_member_get(req.room_id, target_user_id).await?;
            }

            if is_new_join {
                let user = srv.users.get(res.user_id, None).await?;
                s.broadcast_room(
                    req.room_id,
                    auth.user.id,
                    MessageSync::RoomMemberCreate {
                        member: res.clone(),
                        user,
                    },
                )
                .await?;
            }

            return Ok(Json(res));
        }
    }

    let mut perms = s
        .services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?;
    let mut perms: Permissions2<CheckPermissions> = perms.ensure_view()?;
    perms.needs(Permission::IntegrationsBridge);
    perms.check()?;
    let auth_user = srv.users.get(auth.user.id, None).await?;
    let target_user = srv.users.get(target_user_id, None).await?;
    let Some(puppet) = target_user.puppet else {
        return Err(ApiError::from_code(ErrorCode::CantAddThatUser).into());
    };
    if !auth_user.bot {
        return Err(ApiError::from_code(ErrorCode::OnlyBotsCanUseThis).into());
    };

    let app = s
        .data()
        .application_get(auth.user.id.into_inner().into())
        .await?;
    if app.bridge.is_none() {
        return Err(ApiError::from_code(ErrorCode::BotIsNotABridge).into());
    }

    if puppet.owner_id.into_inner() != *auth.user.id {
        return Err(ApiError::from_code(ErrorCode::NotPuppetOwner).into());
    }

    let d = s.data();
    let existing = d.room_member_get(req.room_id, target_user_id).await;

    if let Ok(start) = &existing {
        if req.member.mute.is_some_and(|m| m != start.mute) {
            perms.needs(Permission::VoiceMute);
        }

        if req.member.deaf.is_some_and(|m| m != start.deaf) {
            perms.needs(Permission::VoiceDeafen);
        }

        if req.member.override_name.is_some() && req.member.override_name != start.override_name {
            perms.needs(Permission::MemberNicknameManage);
        }

        if let Some(r) = &mut req.member.roles {
            r.sort();
            perms.needs(Permission::RoleApply);
            let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
            let new = HashSet::<RoleId>::from_iter(r.iter().copied());
            let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;

            // removed roles
            for role_id in old.difference(&new) {
                let role = d.role_select(req.room_id, *role_id).await?;
                if role.position >= rank {
                    return Err(
                        ApiError::from_code(ErrorCode::CannotRemoveRoleAboveYourRole).into(),
                    );
                }
            }

            // added roles
            for role_id in new.difference(&old) {
                let role = d.role_select(req.room_id, *role_id).await?;
                if role.position >= rank {
                    return Err(ApiError::from_code(ErrorCode::CannotAddRoleAboveYourRole).into());
                }
            }
        }
    } else {
        if req.member.mute == Some(true) {
            perms.needs(Permission::VoiceMute);
        }

        if req.member.deaf == Some(true) {
            perms.needs(Permission::VoiceDeafen);
        }

        if req.member.override_name.is_some() {
            perms.needs(Permission::MemberNicknameManage);
        }

        if let Some(r) = &mut req.member.roles {
            r.sort();
            perms.needs(Permission::RoleApply);
            let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
            for role_id in r {
                let role = d.role_select(req.room_id, *role_id).await?;
                if role.position >= rank {
                    return Err(ApiError::from_code(ErrorCode::CannotAddRoleAboveYourRole).into());
                }
            }
        }
    }

    let origin = RoomMemberOrigin::Bridged {
        bridge_id: auth.user.id,
    };
    d.room_member_put(
        req.room_id,
        target_user_id,
        Some(origin),
        req.member.clone(),
    )
    .await?;

    if let Some(r) = req.member.roles {
        if let Ok(start) = &existing {
            let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
            let new = HashSet::<RoleId>::from_iter(r.iter().copied());
            // removed roles
            for role_id in old.difference(&new) {
                d.role_member_delete(req.room_id, target_user_id, *role_id)
                    .await?;
            }

            // added roles
            for role_id in new.difference(&old) {
                d.role_member_put(req.room_id, target_user_id, *role_id)
                    .await?;
            }
        } else {
            for role_id in r {
                d.role_member_put(req.room_id, target_user_id, role_id)
                    .await?;
            }
        }
    }

    s.services()
        .perms
        .invalidate_room(target_user_id, req.room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let mut res = d.room_member_get(req.room_id, target_user_id).await?;

    // scan member with automod
    let automod = srv.automod.load(req.room_id).await?;
    let scan = automod.scan_member(&res, &auth.user);

    let has_block_action = scan
        .actions()
        .iter()
        .any(|action| matches!(action, AutomodAction::Block { .. }));

    if has_block_action {
        d.room_member_set_quarantined(req.room_id, target_user_id, true)
            .await?;
    } else if res.quarantined {
        d.room_member_set_quarantined(req.room_id, target_user_id, false)
            .await?;
    }

    if has_block_action || (!has_block_action && res.quarantined) {
        res = d.room_member_get(req.room_id, target_user_id).await?;
    }

    let changes = if let Ok(existing) = &existing {
        Changes::new()
            .change("override_name", &existing.override_name, &res.override_name)
            .change(
                "override_description",
                &existing.override_description,
                &res.override_description,
            )
            .change("mute", &existing.mute, &res.mute)
            .change("deaf", &existing.deaf, &res.deaf)
            .change("roles", &existing.roles, &res.roles)
    } else {
        Changes::new()
            .add("override_name", &res.override_name)
            .add("override_description", &res.override_description)
            .add("mute", &res.mute)
            .add("deaf", &res.deaf)
            .add("roles", &res.roles)
    };

    let changes = changes.build();
    if !changes.is_empty() {
        al.commit_success(AuditLogEntryType::MemberUpdate {
            room_id: req.room_id,
            user_id: target_user_id,
            changes,
        })
        .await?;

        if existing.is_err() {
            let user = srv.users.get(res.user_id, None).await?;
            s.broadcast_room(
                req.room_id,
                auth.user.id,
                MessageSync::RoomMemberCreate {
                    member: res.clone(),
                    user,
                },
            )
            .await?;
        } else {
            let user = srv.users.get(res.user_id, None).await?;
            s.broadcast_room(
                req.room_id,
                auth.user.id,
                MessageSync::RoomMemberUpdate {
                    member: res.clone(),
                    user,
                },
            )
            .await?;
        }
    }

    Ok(Json(res))
}

/// Room member update
#[handler(routes::room_member_update)]
async fn room_member_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_update::Request,
) -> Result<impl IntoResponse> {
    let mut req = req;
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let al = auth.audit_log(req.room_id);
    req.patch.validate()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let mut perms = s
        .services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?;
    let mut perms: Permissions2<CheckPermissions> = perms.ensure_view()?;
    let srv = s.services();

    // FIXME: allow editing self nickname (override_name)
    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let start = d.room_member_get(req.room_id, target_user_id).await?;
    if !req.patch.changes(&start) {
        return Ok(Json(start));
    }
    if req.patch.mute.is_some_and(|m| m != start.mute) {
        perms.needs(Permission::VoiceMute);
    }
    if req.patch.deaf.is_some_and(|m| m != start.deaf) {
        perms.needs(Permission::VoiceDeafen);
    }
    if req
        .patch
        .override_name
        .as_ref()
        .is_some_and(|m| m != &start.override_name)
    {
        if target_user_id == auth.user.id {
            perms.needs(Permission::MemberNickname);
        } else {
            perms.needs(Permission::MemberNicknameManage);
        }
    }

    if req
        .patch
        .timeout_until
        .as_ref()
        .is_some_and(|val| val != &start.timeout_until)
    {
        perms.needs(Permission::MemberTimeout);
        let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
        let other_rank = srv.perms.get_user_rank(req.room_id, target_user_id).await?;
        let room = srv.rooms.get(req.room_id, None).await?;
        if room.owner_id != Some(auth.user.id) && rank <= other_rank {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
    }

    // TODO: run futures concurrently
    if let Some(r) = &mut req.patch.roles {
        r.sort();
        perms.needs(Permission::RoleApply);
        let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
        let new = HashSet::<RoleId>::from_iter(r.iter().copied());
        let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;

        // removed roles
        for role_id in old.difference(&new) {
            let role = d.role_select(req.room_id, *role_id).await?;
            if role.position >= rank {
                return Err(ApiError::from_code(ErrorCode::CannotRemoveRoleAboveYourRole).into());
            }
        }

        // added roles
        for role_id in new.difference(&old) {
            let role = d.role_select(req.room_id, *role_id).await?;
            if role.position >= rank {
                return Err(ApiError::from_code(ErrorCode::CannotAddRoleAboveYourRole).into());
            }
        }

        // removed roles
        for role_id in old.difference(&new) {
            d.role_member_delete(req.room_id, target_user_id, *role_id)
                .await?;
        }

        // added roles
        for role_id in new.difference(&old) {
            d.role_member_put(req.room_id, target_user_id, *role_id)
                .await?;
        }
    }

    perms.check()?;

    d.room_member_patch(req.room_id, target_user_id, req.patch.clone())
        .await?;
    srv.perms.invalidate_room(target_user_id, req.room_id).await;

    if req
        .patch
        .timeout_until
        .as_ref()
        .is_some_and(|val| val != &start.timeout_until)
    {
        srv.perms
            .update_timeout_task(
                target_user_id,
                req.room_id,
                req.patch.timeout_until.flatten(),
            )
            .await;
    }

    let mut res = d.room_member_get(req.room_id, target_user_id).await?;

    // scan member with automod
    let automod = srv.automod.load(req.room_id).await?;
    let scan = automod.scan_member(&res, &auth.user);

    let has_block_action = scan
        .actions()
        .iter()
        .any(|action| matches!(action, AutomodAction::Block { .. }));

    if has_block_action {
        d.room_member_set_quarantined(req.room_id, target_user_id, true)
            .await?;
    } else if res.quarantined {
        d.room_member_set_quarantined(req.room_id, target_user_id, false)
            .await?;
    }

    if has_block_action || (!has_block_action && res.quarantined) {
        res = d.room_member_get(req.room_id, target_user_id).await?;
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

    if !changes.is_empty() {
        al.commit_success(AuditLogEntryType::MemberUpdate {
            room_id: req.room_id,
            user_id: target_user_id,
            changes,
        })
        .await?;
    }

    let user = srv.users.get(res.user_id, None).await?;
    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::RoomMemberUpdate {
            member: res.clone(),
            user,
        },
    )
    .await?;
    Ok(Json(res))
}

/// Room member delete (kick/leave)
#[handler(routes::room_member_delete)]
async fn room_member_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();

    // FIXME: allow leaving rooms
    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?;
    if target_user_id != auth.user.id {
        perms.needs(Permission::MemberKick);
    }
    perms.check()?;

    if req.room_id == SERVER_ROOM_ID {
        return Err(ApiError::from_code(ErrorCode::CannotKickFromServerRoom).into());
    }
    let room = srv.rooms.get(req.room_id, None).await?;
    if room.owner_id == Some(target_user_id) {
        return Err(ApiError::from_code(ErrorCode::RoomOwnerCannotLeave).into());
    }
    if auth.user.id != target_user_id {
        if room.owner_id != Some(auth.user.id) {
            let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
            let other_rank = srv.perms.get_user_rank(req.room_id, target_user_id).await?;
            if rank <= other_rank {
                return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
            }
        }
    }
    if room.owner_id == Some(target_user_id) {
        return Err(ApiError::from_code(ErrorCode::CannotBanRoomOwner).into());
    }
    d.room_member_leave(req.room_id, target_user_id).await?;
    srv.perms.invalidate_room(target_user_id, req.room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::MemberKick {
        room_id: req.room_id,
        user_id: target_user_id,
    })
    .await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::RoomMemberDelete {
            room_id: req.room_id,
            user_id: target_user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Room member search
#[handler(routes::room_member_search)]
async fn room_member_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_search::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let d = s.data();
    let mut perms = s
        .services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?;

    // extra permission check to prevent returning the entire list of registered users
    if req.room_id == SERVER_ROOM_ID {
        perms.needs(Permission::ServerOversee);
    }
    perms.check()?;

    let limit = req.search.limit.unwrap_or(10).min(100);

    let room_members = d
        .room_member_search(req.room_id, req.search.query, limit)
        .await?;

    let user_ids: Vec<UserId> = room_members.iter().map(|m| m.user_id).collect();

    let users = s.services().users.get_many(&user_ids).await?;

    Ok(Json(RoomMemberSearchResponse {
        room_members,
        users,
    }))
}

/// Room member search advanced
#[handler(routes::room_member_search_advanced)]
async fn room_member_search_advanced(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_member_search_advanced::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    req.search.validate()?;
    let d = s.data();
    let mut perms = s
        .services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?;

    // extra permission check to prevent returning the entire list of registered users
    if req.room_id == SERVER_ROOM_ID {
        perms.needs(Permission::ServerOversee);
    }
    perms.check()?;

    let res = d
        .room_member_search_advanced(req.room_id, req.search)
        .await?;

    Ok(Json(res))
}

/// Room member prune (TODO)
///
/// bulk remove users. useful for keping a room's member count below the room member limit.
#[handler(routes::room_prune_begin)]
async fn room_prune_begin(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_prune_begin::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.prune.validate()?;
    let srv = s.services();
    let data = s.data();

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = s
        .services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::MemberKick);
    perms.needs(Permission::RoomEdit);
    perms.check()?;

    Ok(Error::Unimplemented)
}

/// Room ban create
#[handler(routes::room_ban_create)]
async fn room_ban_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_ban_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let srv = s.services();
    let d = s.data();

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    let mut perms = srv
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::MemberBan);
    perms.check()?;
    if req.room_id == SERVER_ROOM_ID {
        return Err(ApiError::from_code(ErrorCode::CannotKickFromServerRoom).into());
    }

    // enforce ranking if you're banning a member
    if let Ok(_member) = d.room_member_get(req.room_id, target_user_id).await {
        let room = srv.rooms.get(req.room_id, None).await?;
        if room.owner_id != Some(auth.user.id) {
            let rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;
            let other_rank = srv.perms.get_user_rank(req.room_id, target_user_id).await?;
            if rank <= other_rank {
                return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
            }
        }
        if room.owner_id == Some(target_user_id) {
            return Err(ApiError::from_code(ErrorCode::CannotBanRoomOwner).into());
        }
    }

    d.room_ban_create(
        req.room_id,
        target_user_id,
        auth.reason.clone(),
        req.ban.expires_at,
    )
    .await?;
    let ban = d.room_ban_get(req.room_id, target_user_id).await?;
    srv.perms.invalidate_room(target_user_id, req.room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);
    d.room_member_leave(req.room_id, target_user_id).await?;

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::MemberBan {
        room_id: req.room_id,
        user_id: target_user_id,
    })
    .await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::RoomMemberDelete {
            room_id: req.room_id,
            user_id: target_user_id,
        },
    )
    .await?;
    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::BanCreate {
            room_id: req.room_id,
            ban,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room ban create bulk
#[handler(routes::room_ban_bulk_create)]
async fn room_ban_bulk_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_ban_bulk_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.ban.validate()?;
    let srv = s.services();
    let d = s.data();

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;
    if req.room_id == SERVER_ROOM_ID {
        return Err(ApiError::from_code(ErrorCode::CannotKickFromServerRoom).into());
    }

    let room = srv.rooms.get(req.room_id, None).await?;
    let auth_user_rank = srv.perms.get_user_rank(req.room_id, auth.user.id).await?;

    for &target_user_id in &req.ban.target_ids {
        if let Ok(_member) = d.room_member_get(req.room_id, target_user_id).await {
            if room.owner_id != Some(auth.user.id) {
                let other_rank = srv.perms.get_user_rank(req.room_id, target_user_id).await?;
                if auth_user_rank <= other_rank {
                    return Err(ApiError::from_code(ErrorCode::InsufficientRankToManageUser).into());
                }
            }
        }
    }

    d.room_ban_create_bulk(
        req.room_id,
        &req.ban.target_ids,
        auth.reason.clone(),
        req.ban.expires_at,
    )
    .await?;

    for &target_user_id in &req.ban.target_ids {
        srv.perms.invalidate_room(target_user_id, req.room_id).await;
        srv.perms.invalidate_is_mutual(target_user_id);
        d.room_member_leave(req.room_id, target_user_id).await?;

        let al = auth.audit_log(req.room_id);
        al.commit_success(AuditLogEntryType::MemberBan {
            room_id: req.room_id,
            user_id: target_user_id,
        })
        .await?;

        s.broadcast_room(
            req.room_id,
            auth.user.id,
            MessageSync::RoomMemberDelete {
                room_id: req.room_id,
                user_id: target_user_id,
            },
        )
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Room ban remove
#[handler(routes::room_ban_delete)]
async fn room_ban_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_ban_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;

    d.room_ban_delete(req.room_id, target_user_id).await?;
    srv.perms.invalidate_room(target_user_id, req.room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::MemberUnban {
        room_id: req.room_id,
        user_id: target_user_id,
    })
    .await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::BanDelete {
            room_id: req.room_id,
            user_id: target_user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Room ban get
#[handler(routes::room_ban_get)]
async fn room_ban_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_ban_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    s.services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;
    let res = d.room_ban_get(req.room_id, target_user_id).await?;
    Ok(Json(res))
}

/// Room ban list
#[handler(routes::room_ban_list)]
async fn room_ban_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_ban_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let d = s.data();
    s.services()
        .perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;
    let res = d.room_ban_list(req.room_id, req.pagination).await?;
    let cursor = res.items.last().map(|i| i.user_id.to_string());
    let res = PaginationResponse {
        items: res.items,
        has_more: res.has_more,
        total: res.total,
        cursor,
    };
    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(room_member_list))
        .routes(routes2!(room_member_get))
        .routes(routes2!(room_member_add))
        .routes(routes2!(room_member_update))
        .routes(routes2!(room_member_delete))
        .routes(routes2!(room_member_search))
        .routes(routes2!(room_member_search_advanced))
        .routes(routes2!(room_prune_begin))
        .routes(routes2!(room_ban_create))
        .routes(routes2!(room_ban_bulk_create))
        .routes(routes2!(room_ban_delete))
        .routes(routes2!(room_ban_get))
        .routes(routes2!(room_ban_list))
}
