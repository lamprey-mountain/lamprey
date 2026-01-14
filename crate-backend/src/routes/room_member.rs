use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::{Diff, Time};
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, PaginationQuery,
    PaginationResponse, Permission, PruneBegin, PruneResponse, RoomId, RoomMember, RoomMemberPatch,
    RoomMemberPut, RoomMemberSearch, RoomMemberSearchAdvanced, RoomMemberSearchResponse,
    RoomMembership, UserId,
};
use common::v1::types::{
    RoleId, RoomBan, RoomBanBulkCreate, RoomBanCreate, RoomMemberOrigin, SERVER_ROOM_ID,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct RoomBanSearchQuery {
    pub query: String,
}

/// Room member list
#[utoipa::path(
    get,
    path = "/room/{room_id}/member",
    params(
        PaginationQuery<UserId>,
        ("room_id" = RoomId, description = "Room id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = PaginationResponse<RoomMember>, description = "success"),
    )
)]
async fn room_member_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;

    // extra permission check to prevent returning the entire list of registered users
    if room_id == SERVER_ROOM_ID {
        perms.ensure(Permission::ServerOversee)?;
    }

    let res = d.room_member_list(room_id, paginate).await?;
    Ok(Json(res))
}

/// Room member get
#[utoipa::path(
    get,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
    )
)]
async fn room_member_get(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let _perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    let res = d.room_member_get(room_id, target_user_id).await?;
    if res.membership == RoomMembership::Join {
        Ok(Json(res))
    } else {
        Err(Error::NotFound)
    }
}

// FIXME: only return 304 not modified if an etag is sent
/// Room member add
///
/// - Bots can add puppet users via MemberBridge permission
/// - Users can join public rooms by specifying themselves as the target
/// - Only registered users (not guests) can join public rooms
#[utoipa::path(
    put,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = [
        "room_member",
        "badge.perm.MemberBridge",
        "badge.perm-opt.VoiceMute",
        "badge.perm-opt.VoiceDeafen",
        "badge.perm-opt.MemberManage",
        "badge.perm-opt.RoleApply",
        "badge.room-mfa-opt",
    ],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn room_member_add(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(mut json): Json<RoomMemberPut>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

    // allow self joins
    if room.security.require_mfa && target_user_id != auth.user.id {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    // handle joining public rooms
    if target_user_id == auth.user.id {
        let room = srv.rooms.get(room_id, None).await?;
        if room.public {
            if auth.user.registered_at.is_none() {
                return Err(Error::BadStatic("guests cannot join public rooms"));
            }

            if let Ok(ban) = s.data().room_ban_get(room_id, target_user_id).await {
                if let Some(expires_at) = ban.expires_at {
                    if expires_at > Time::now_utc() {
                        return Err(Error::BadStatic("banned"));
                    }
                } else {
                    return Err(Error::BadStatic("banned"));
                }
            }

            let d = s.data();
            let existing = d.room_member_get(room_id, target_user_id).await;
            let perms = if existing.is_ok() {
                // User already exists, get their actual permissions
                s.services().perms.for_room(auth.user.id, room_id).await?
            } else {
                // User doesn't exist yet, get default room permissions
                s.services().perms.default_for_room(room_id).await?
            };

            if let Ok(start) = &existing {
                // already exists
                if json.mute.is_some_and(|m| m != start.mute) {
                    perms.ensure(Permission::VoiceMute)?;
                }
                if json.deaf.is_some_and(|m| m != start.deaf) {
                    perms.ensure(Permission::VoiceDeafen)?;
                }
                if json.override_name.is_some() && json.override_name != start.override_name {
                    perms.ensure(Permission::MemberNickname)?;
                }
                if let Some(r) = &mut json.roles {
                    // TODO: let users add self applicable roles to themselves
                    // TODO: also handle if @everyone has RoleApply permissions
                    if !r.is_empty() {
                        return Err(Error::BadStatic("cannot add roles to yourself"));
                    }
                }
            } else {
                // joining for the first time
                if json.mute == Some(true) {
                    perms.ensure(Permission::VoiceMute)?;
                }
                if json.deaf == Some(true) {
                    perms.ensure(Permission::VoiceDeafen)?;
                }
                if json.override_name.is_some() {
                    perms.ensure(Permission::MemberNickname)?;
                }
                if let Some(r) = &mut json.roles {
                    // TODO: let users add self applicable roles to themselves
                    // TODO: also handle if @everyone has RoleApply permissions
                    if !r.is_empty() {
                        return Err(Error::BadStatic("cannot add roles to yourself"));
                    }
                }
            }

            let origin = RoomMemberOrigin::PublicJoin;
            d.room_member_put(room_id, target_user_id, Some(origin), json.clone())
                .await?;

            s.services()
                .perms
                .invalidate_room(target_user_id, room_id)
                .await;
            s.services().perms.invalidate_is_mutual(target_user_id);
            let res = d.room_member_get(room_id, target_user_id).await?;

            let is_new_join = existing.is_err();

            // handle role updates if any
            if let Some(r) = json.roles {
                if let Ok(ref existing) = existing {
                    let old = HashSet::<RoleId>::from_iter(existing.roles.iter().copied());
                    let new = HashSet::<RoleId>::from_iter(r.iter().copied());
                    // removed roles
                    for role_id in old.difference(&new) {
                        d.role_member_delete(room_id, target_user_id, *role_id)
                            .await?;
                    }

                    // added roles
                    for role_id in new.difference(&old) {
                        d.role_member_put(room_id, target_user_id, *role_id).await?;
                    }
                } else {
                    for role_id in r {
                        d.role_member_put(room_id, target_user_id, role_id).await?;
                    }
                }
            }

            if is_new_join {
                s.broadcast_room(
                    room_id,
                    auth.user.id,
                    MessageSync::RoomMemberCreate {
                        member: res.clone(),
                    },
                )
                .await?;
                s.broadcast_room(
                    room_id,
                    auth.user.id,
                    MessageSync::RoomMemberUpsert {
                        member: res.clone(),
                    },
                )
                .await?;
            }

            return Ok(Json(res));
        }
    }

    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBridge)?;
    let auth_user = srv.users.get(auth.user.id, None).await?;
    let target_user = srv.users.get(target_user_id, None).await?;
    let Some(puppet) = target_user.puppet else {
        return Err(Error::BadStatic("can't add that user"));
    };
    let Some(bot) = auth_user.bot else {
        return Err(Error::BadStatic("only bots can use this"));
    };
    if !bot.is_bridge {
        return Err(Error::BadStatic("bot is not a bridge"));
    }

    if puppet.owner_id != auth.user.id {
        return Err(Error::BadStatic("not puppet owner"));
    }

    let d = s.data();
    let existing = d.room_member_get(room_id, target_user_id).await;

    if let Ok(start) = &existing {
        if json.mute.is_some_and(|m| m != start.mute) {
            perms.ensure(Permission::VoiceMute)?;
        }

        if json.deaf.is_some_and(|m| m != start.deaf) {
            perms.ensure(Permission::VoiceDeafen)?;
        }

        if json.override_name.is_some() && json.override_name != start.override_name {
            perms.ensure(Permission::MemberNicknameManage)?;
        }

        if let Some(r) = &mut json.roles {
            r.sort();
            perms.ensure(Permission::RoleApply)?;
            let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
            let new = HashSet::<RoleId>::from_iter(r.iter().copied());
            let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;

            // removed roles
            for role_id in old.difference(&new) {
                let role = d.role_select(room_id, *role_id).await?;
                if role.position >= rank {
                    return Err(Error::BadStatic("cannot remove role above your role"));
                }
            }

            // added roles
            for role_id in new.difference(&old) {
                let role = d.role_select(room_id, *role_id).await?;
                if role.position >= rank {
                    return Err(Error::BadStatic("cannot add role above your role"));
                }
            }
        }
    } else {
        if json.mute == Some(true) {
            perms.ensure(Permission::VoiceMute)?;
        }

        if json.deaf == Some(true) {
            perms.ensure(Permission::VoiceDeafen)?;
        }

        if json.override_name.is_some() {
            perms.ensure(Permission::MemberNicknameManage)?;
        }

        if let Some(r) = &mut json.roles {
            r.sort();
            perms.ensure(Permission::RoleApply)?;
            let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
            for role_id in r {
                let role = d.role_select(room_id, *role_id).await?;
                if role.position >= rank {
                    return Err(Error::BadStatic("cannot add role above your role"));
                }
            }
        }
    }

    let origin = RoomMemberOrigin::Bridged {
        bridge_id: auth.user.id,
    };
    d.room_member_put(room_id, target_user_id, Some(origin), json.clone())
        .await?;

    if let Some(r) = json.roles {
        if let Ok(start) = &existing {
            let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
            let new = HashSet::<RoleId>::from_iter(r.iter().copied());
            // removed roles
            for role_id in old.difference(&new) {
                d.role_member_delete(room_id, target_user_id, *role_id)
                    .await?;
            }

            // added roles
            for role_id in new.difference(&old) {
                d.role_member_put(room_id, target_user_id, *role_id).await?;
            }
        } else {
            for role_id in r {
                d.role_member_put(room_id, target_user_id, role_id).await?;
            }
        }
    }

    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;

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
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::MemberUpdate {
                room_id,
                user_id: target_user_id,
                changes,
            },
        })
        .await?;

        if existing.is_err() {
            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::RoomMemberCreate {
                    member: res.clone(),
                },
            )
            .await?;
        } else {
            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::RoomMemberUpdate {
                    member: res.clone(),
                },
            )
            .await?;
        }

        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::RoomMemberUpsert {
                member: res.clone(),
            },
        )
        .await?;
    }

    Ok(Json(res))
}

/// Room member update
#[utoipa::path(
    patch,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = [
        "room_member",
        "badge.perm-opt.VoiceMute",
        "badge.perm-opt.VoiceDeafen",
        "badge.perm-opt.MemberManage",
        "badge.perm-opt.RoleApply",
        "badge.room-mfa",
    ],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn room_member_update(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(mut json): Json<RoomMemberPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    let srv = s.services();

    // FIXME: allow editing self
    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Ok(Json(start));
    }
    if !json.changes(&start) {
        return Ok(Json(start));
    }
    if json.mute.is_some_and(|m| m != start.mute) {
        perms.ensure(Permission::VoiceMute)?;
    }
    if json.deaf.is_some_and(|m| m != start.deaf) {
        perms.ensure(Permission::VoiceDeafen)?;
    }
    if json
        .override_name
        .as_ref()
        .is_some_and(|m| m != &start.override_name)
    {
        perms.ensure(Permission::MemberNicknameManage)?;
    }

    if json.timeout_until.changes(&start.timeout_until) {
        perms.ensure(Permission::MemberTimeout)?;
        let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
        let other_rank = srv.perms.get_user_rank(room_id, target_user_id).await?;
        let room = srv.rooms.get(room_id, None).await?;
        if room.owner_id != Some(auth.user.id) && rank <= other_rank {
            return Err(Error::BadStatic("your rank is too low"));
        }
    }

    // TODO: run futures concurrently
    if let Some(r) = &mut json.roles {
        r.sort();
        perms.ensure(Permission::RoleApply)?;
        let old = HashSet::<RoleId>::from_iter(start.roles.iter().copied());
        let new = HashSet::<RoleId>::from_iter(r.iter().copied());
        let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;

        // removed roles
        for role_id in old.difference(&new) {
            let role = d.role_select(room_id, *role_id).await?;
            if role.position >= rank {
                return Err(Error::BadStatic("cannot remove role above your role"));
            }
        }

        // added roles
        for role_id in new.difference(&old) {
            let role = d.role_select(room_id, *role_id).await?;
            if role.position >= rank {
                return Err(Error::BadStatic("cannot add role above your role"));
            }
        }

        // removed roles
        for role_id in old.difference(&new) {
            d.role_member_delete(room_id, target_user_id, *role_id)
                .await?;
        }

        // added roles
        for role_id in new.difference(&old) {
            d.role_member_put(room_id, target_user_id, *role_id).await?;
        }
    }

    d.room_member_patch(room_id, target_user_id, json.clone())
        .await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;

    if json.timeout_until.changes(&start.timeout_until) {
        srv.perms
            .update_timeout_task(target_user_id, room_id, json.timeout_until.flatten())
            .await;
    }

    let res = d.room_member_get(room_id, target_user_id).await?;

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
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::MemberUpdate {
                room_id,
                user_id: target_user_id,
                changes,
            },
        })
        .await?;
    }

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoomMemberUpdate {
            member: res.clone(),
        },
    )
    .await?;
    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoomMemberUpsert {
            member: res.clone(),
        },
    )
    .await?;
    Ok(Json(res))
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
struct LeaveQuery {
    /// when leaving a room, allow this room to be found with ?include=Removed
    #[serde(default)]
    soft: bool,
    // /// don't send any leave messages?
    // // wasn't planning on doing it for rooms anyways, maybe threads though?
    // #[serde(default)]
    // silent: bool,
}

/// Room member delete (kick/leave)
#[utoipa::path(
    delete,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member", "badge.perm-opt.MemberKick", "badge.room-mfa-opt"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_member_delete(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    Query(_q): Query<LeaveQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();

    // FIXME: allow leaving rooms
    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    if target_user_id != auth.user.id {
        perms.ensure(Permission::MemberKick)?;
    }
    if room_id == SERVER_ROOM_ID {
        return Err(Error::BadStatic("cannot kick people from the server room"));
    }
    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    let room = srv.rooms.get(room_id, None).await?;
    if room.owner_id == Some(target_user_id) {
        return Err(Error::BadStatic("room owner cannot leave the room"));
    }
    if auth.user.id != target_user_id {
        if room.owner_id != Some(auth.user.id) {
            let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
            let other_rank = srv.perms.get_user_rank(room_id, target_user_id).await?;
            if rank <= other_rank {
                return Err(Error::BadStatic("your rank is too low"));
            }
        }
    }
    if room.owner_id == Some(target_user_id) {
        return Err(Error::BadStatic("cannot ban room owner"));
    }
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave {})
        .await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberKick {
            room_id,
            user_id: target_user_id,
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoomMemberDelete {
            room_id,
            user_id: target_user_id,
        },
    )
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoomMemberUpsert { member: res },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room member search
#[utoipa::path(
    get,
    path = "/room/{room_id}/member/search",
    params(
        RoomMemberSearch,
        ("room_id" = RoomId, description = "Room id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMemberSearchResponse, description = "success"),
    )
)]
async fn room_member_search(
    Path(room_id): Path<RoomId>,
    Query(search): Query<RoomMemberSearch>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;

    // extra permission check to prevent returning the entire list of registered users
    if room_id == SERVER_ROOM_ID {
        perms.ensure(Permission::ServerOversee)?;
    }

    let limit = search.limit.unwrap_or(10).min(100);

    let room_members = d.room_member_search(room_id, search.query, limit).await?;

    let user_ids: Vec<UserId> = room_members.iter().map(|m| m.user_id).collect();

    let users = s.services().users.get_many(&user_ids).await?;

    Ok(Json(RoomMemberSearchResponse {
        room_members,
        users,
    }))
}

/// Room member search advanced
#[utoipa::path(
    post,
    path = "/room/{room_id}/member/search",
    request_body = RoomMemberSearchAdvanced,
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMemberSearchResponse, description = "success"),
    )
)]
async fn room_member_search_advanced(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(search): Json<RoomMemberSearchAdvanced>,
) -> Result<impl IntoResponse> {
    search.validate()?;
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;

    // extra permission check to prevent returning the entire list of registered users
    if room_id == SERVER_ROOM_ID {
        perms.ensure(Permission::ServerOversee)?;
    }

    let res = d.room_member_search_advanced(room_id, search).await?;

    Ok(Json(res))
}

/// Room member prune (TODO)
///
/// bulk remove users. useful for keping a room's member count below the room member limit.
#[utoipa::path(
    post,
    path = "/room/{room_id}/prune",
    request_body = PruneBegin,
    tags = ["room_member", "badge.perm.MemberKick", "badge.perm.RoomManage", "badge.room-mfa"],
    responses(
        (status = OK, body = PruneResponse, description = "success"),
        (status = ACCEPTED, description = "prune started"),
    )
)]
async fn room_member_prune(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PruneBegin>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let data = s.data();

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberKick)?;
    perms.ensure(Permission::RoomManage)?;

    Ok(Error::Unimplemented)
}

/// Room ban create
#[utoipa::path(
    put,
    path = "/room/{room_id}/ban/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member", "badge.perm.MemberBan", "badge.room-mfa"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_ban_create(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(create): Json<RoomBanCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let srv = s.services();
    let d = s.data();

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;
    if room_id == SERVER_ROOM_ID {
        return Err(Error::BadStatic("cannot kick people from the server room"));
    }

    // enforce ranking if you're banning a member
    if let Ok(member) = d.room_member_get(room_id, target_user_id).await {
        let room = srv.rooms.get(room_id, None).await?;
        if room.owner_id != Some(auth.user.id) && member.membership == RoomMembership::Join {
            let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
            let other_rank = srv.perms.get_user_rank(room_id, target_user_id).await?;
            if rank <= other_rank {
                return Err(Error::BadStatic("your rank is too low"));
            }
        }
        if room.owner_id == Some(target_user_id) {
            return Err(Error::BadStatic("cannot ban room owner"));
        }
    }

    d.room_ban_create(room_id, target_user_id, reason.clone(), create.expires_at)
        .await?;
    let ban = d.room_ban_get(room_id, target_user_id).await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave)
        .await?;
    let member = d.room_member_get(room_id, target_user_id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberBan {
            room_id,
            user_id: target_user_id,
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoomMemberDelete {
            room_id,
            user_id: target_user_id,
        },
    )
    .await?;
    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::RoomMemberUpsert { member },
    )
    .await?;
    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::BanCreate { room_id, ban },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room ban create bulk
#[utoipa::path(
    post,
    path = "/room/{room_id}/ban",
    tags = ["room_member", "badge.perm.MemberBan", "badge.room-mfa"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn room_ban_create_bulk(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(create): Json<RoomBanBulkCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    create.validate()?;
    let srv = s.services();
    let d = s.data();

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;
    if room_id == SERVER_ROOM_ID {
        return Err(Error::BadStatic("cannot kick people from the server room"));
    }

    let room = srv.rooms.get(room_id, None).await?;
    let auth_user_rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;

    for &target_user_id in &create.target_ids {
        if let Ok(member) = d.room_member_get(room_id, target_user_id).await {
            if room.owner_id != Some(auth.user.id) && member.membership == RoomMembership::Join {
                let other_rank = srv.perms.get_user_rank(room_id, target_user_id).await?;
                if auth_user_rank <= other_rank {
                    return Err(Error::BadStatic(
                        "your rank is too low to ban one of the users",
                    ));
                }
            }
        }
    }

    d.room_ban_create_bulk(
        room_id,
        &create.target_ids,
        reason.clone(),
        create.expires_at,
    )
    .await?;

    for &target_user_id in &create.target_ids {
        srv.perms.invalidate_room(target_user_id, room_id).await;
        srv.perms.invalidate_is_mutual(target_user_id);
        d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave)
            .await?;
        let member = d.room_member_get(room_id, target_user_id).await?;

        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::MemberBan {
                room_id,
                user_id: target_user_id,
            },
        })
        .await?;

        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::RoomMemberDelete {
                room_id,
                user_id: target_user_id,
            },
        )
        .await?;

        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::RoomMemberUpsert { member },
        )
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Room ban remove
#[utoipa::path(
    delete,
    path = "/room/{room_id}/ban/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member", "badge.perm.MemberBan", "badge.room-mfa"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_ban_remove(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = d.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("mfa required for this action"));
        }
    }

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;

    d.room_ban_delete(room_id, target_user_id).await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberUnban {
            room_id,
            user_id: target_user_id,
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::BanDelete {
            room_id,
            user_id: target_user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Room ban get
#[utoipa::path(
    get,
    path = "/room/{room_id}/ban/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member", "badge.perm.MemberBan"],
    responses(
        (status = OK, body = RoomBan, description = "success"),
    )
)]
async fn room_ban_get(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;
    let res = d.room_ban_get(room_id, target_user_id).await?;
    Ok(Json(res))
}

/// Room ban list
#[utoipa::path(
    get,
    path = "/room/{room_id}/ban",
    params(
        PaginationQuery<UserId>,
        ("room_id" = RoomId, description = "Room id"),
    ),
    tags = ["room_member", "badge.perm.MemberBan"],
    responses(
        (status = OK, body = PaginationResponse<RoomBan>, description = "success"),
    )
)]
async fn room_ban_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;
    let res = d.room_ban_list(room_id, paginate).await?;
    let cursor = res.items.last().map(|i| i.user_id.to_string());
    let res = PaginationResponse {
        items: res.items,
        has_more: res.has_more,
        total: res.total,
        cursor,
    };
    Ok(Json(res))
}

/// Room ban search
#[utoipa::path(
    get,
    path = "/room/{room_id}/ban/search",
    params(
        RoomBanSearchQuery,
        PaginationQuery<UserId>,
        ("room_id" = RoomId, description = "Room id"),
    ),
    tags = ["room_member", "badge.perm.MemberBan"],
    responses(
        (status = OK, body = PaginationResponse<RoomBan>, description = "success"),
    )
)]
async fn room_ban_search(
    Path(room_id): Path<RoomId>,
    Query(search): Query<RoomBanSearchQuery>,
    Query(pagination): Query<PaginationQuery<UserId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;
    let res = d.room_ban_search(room_id, search.query, pagination).await?;
    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_member_list))
        .routes(routes!(room_member_get))
        .routes(routes!(room_member_add))
        .routes(routes!(room_member_update))
        .routes(routes!(room_member_delete))
        .routes(routes!(room_member_search))
        .routes(routes!(room_member_search_advanced))
        .routes(routes!(room_member_prune))
        .routes(routes!(room_ban_create_bulk))
        .routes(routes!(room_ban_create))
        .routes(routes!(room_ban_remove))
        .routes(routes!(room_ban_get))
        .routes(routes!(room_ban_list))
        .routes(routes!(room_ban_search))
}
