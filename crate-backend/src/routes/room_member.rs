use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Diff;
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, PaginationQuery,
    PaginationResponse, Permission, RoomId, RoomMember, RoomMemberPatch, RoomMemberPut,
    RoomMembership, UserId,
};
use common::v1::types::{RoomBanBulkCreate, RoomBanCreate, RoomMemberOrigin};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};

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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
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
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
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
/// Only `Puppet` users can be added to rooms (via MemberBridge permission)
#[utoipa::path(
    put,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn room_member_add(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoomMemberPut>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MemberBridge)?;
    let auth_user = srv.users.get(auth_user_id).await?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let target_user = srv.users.get(target_user_id).await?;
    let Some(puppet) = target_user.puppet else {
        return Err(Error::BadStatic("can't add that user"));
    };
    let Some(bot) = auth_user.bot else {
        return Err(Error::BadStatic("only bots can use this"));
    };
    if !bot.is_bridge {
        return Err(Error::BadStatic("bot is not a bridge"));
    }

    if puppet.owner_id != auth_user_id {
        return Err(Error::BadStatic("not puppet owner"));
    }

    let d = s.data();
    let existing = d.room_member_get(room_id, target_user_id).await;
    if let Ok(existing) = &existing {
        if existing.override_name == json.override_name
            && existing.override_description == json.override_description
            && json.mute.is_none_or(|m| m == existing.mute)
            && json.deaf.is_none_or(|m| m == existing.deaf)
        {
            return Err(Error::NotModified);
        }
    } else {
        if json.mute == Some(true) {
            perms.ensure(Permission::VoiceMute)?;
        }
        if json.deaf == Some(true) {
            perms.ensure(Permission::VoiceDeafen)?;
        }
    }

    let origin = RoomMemberOrigin::Bridged {
        bridge_id: auth_user_id,
    };
    d.room_member_put(room_id, target_user_id, origin, json)
        .await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;

    let changes = if let Ok(existing) = existing {
        Changes::new()
            .change("override_name", &existing.override_name, &res.override_name)
            .change(
                "override_description",
                &existing.override_description,
                &res.override_description,
            )
            .change("mute", &existing.mute, &res.mute)
            .change("deaf", &existing.deaf, &res.deaf)
    } else {
        Changes::new()
            .add("override_name", &res.override_name)
            .add("override_description", &res.override_description)
            .add("mute", &res.mute)
            .add("deaf", &res.deaf)
    };

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user_id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberUpdate {
            room_id,
            user_id: target_user_id,
            changes: changes.build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user_id,
        MessageSync::RoomMemberUpsert {
            member: res.clone(),
        },
    )
    .await?;
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
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn room_member_update(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoomMemberPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberManage)?;
    }

    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    if !json.changes(&start) {
        return Err(Error::NotModified);
    }
    if json.mute.is_some_and(|m| m != start.mute) {
        perms.ensure(Permission::VoiceMute)?;
    }
    if json.deaf.is_some_and(|m| m != start.deaf) {
        perms.ensure(Permission::VoiceDeafen)?;
    }

    d.room_member_patch(room_id, target_user_id, json).await?;
    let res = d.room_member_get(room_id, target_user_id).await?;

    let changes = Changes::new()
        .change("override_name", &start.override_name, &res.override_name)
        .change(
            "override_description",
            &start.override_description,
            &res.override_description,
        )
        .change("mute", &start.mute, &res.mute)
        .change("deaf", &start.deaf, &res.deaf);

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user_id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberUpdate {
            room_id,
            user_id: target_user_id,
            changes: changes.build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user_id,
        MessageSync::RoomMemberUpsert {
            member: res.clone(),
        },
    )
    .await?;
    Ok(Json(res).into_response())
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
    tags = ["room_member"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_member_delete(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    Query(_q): Query<LeaveQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberKick)?;
    }
    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    if auth_user_id != target_user_id {
        let room = srv.rooms.get(room_id, None).await?;
        if room.owner_id != Some(auth_user_id) {
            let rank = srv.perms.get_user_rank(room_id, auth_user_id).await?;
            let other_rank = srv.perms.get_user_rank(room_id, target_user_id).await?;
            if rank <= other_rank {
                return Err(Error::BadStatic("your rank is too low"));
            }
        }
    }
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave {})
        .await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user_id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberKick {
            room_id,
            user_id: target_user_id,
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user_id,
        MessageSync::RoomMemberUpsert { member: res },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room ban create
#[utoipa::path(
    put,
    path = "/room/{room_id}/ban/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = UserId, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_ban_create(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(create): Json<RoomBanCreate>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let srv = s.services();
    let d = s.data();
    let perms = srv.perms.for_room(auth_user_id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;

    // enforce ranking if you're banning a member
    if let Ok(member) = d.room_member_get(room_id, target_user_id).await {
        let room = srv.rooms.get(room_id, None).await?;
        if room.owner_id != Some(auth_user_id) && member.membership == RoomMembership::Join {
            let rank = srv.perms.get_user_rank(room_id, auth_user_id).await?;
            let other_rank = srv.perms.get_user_rank(room_id, target_user_id).await?;
            if rank <= other_rank {
                return Err(Error::BadStatic("your rank is too low"));
            }
        }
    }

    d.room_ban_create(room_id, target_user_id, reason.clone(), create.expires_at)
        .await?;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.perms.invalidate_is_mutual(target_user_id);
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave)
        .await?;
    let member = d.room_member_get(room_id, target_user_id).await?;

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user_id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberBan {
            room_id,
            user_id: target_user_id,
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user_id,
        MessageSync::RoomMemberUpsert { member },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Room ban create bulk
#[utoipa::path(
    post,
    path = "/room/{room_id}/ban",
    params(("room_id" = RoomId, description = "Room id")),
    tags = ["room_member"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn room_ban_create_bulk(
    Path(room_id): Path<RoomId>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(create): Json<RoomBanBulkCreate>,
) -> Result<impl IntoResponse> {
    create.validate()?;
    let srv = s.services();
    let d = s.data();
    let perms = srv.perms.for_room(auth_user_id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;

    let room = srv.rooms.get(room_id, None).await?;
    let auth_user_rank = srv.perms.get_user_rank(room_id, auth_user_id).await?;

    for &target_user_id in &create.target_ids {
        if let Ok(member) = d.room_member_get(room_id, target_user_id).await {
            if room.owner_id != Some(auth_user_id) && member.membership == RoomMembership::Join {
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

        d.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user_id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MemberBan {
                room_id,
                user_id: target_user_id,
            },
        })
        .await?;

        s.broadcast_room(
            room_id,
            auth_user_id,
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
    tags = ["room_member"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_ban_remove(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;

    d.room_ban_delete(room_id, target_user_id).await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user_id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::MemberUnban {
            room_id,
            user_id: target_user_id,
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user_id,
        MessageSync::RoomMemberUpsert { member: res },
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
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
    )
)]
async fn room_ban_get(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
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
    tags = ["room_member"],
    responses(
        (status = OK, body = PaginationResponse<RoomMember>, description = "success"),
    )
)]
async fn room_ban_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_member_list))
        .routes(routes!(room_member_get))
        .routes(routes!(room_member_add))
        .routes(routes!(room_member_update))
        .routes(routes!(room_member_delete))
        .routes(routes!(room_ban_create_bulk))
        .routes(routes!(room_ban_create))
        .routes(routes!(room_ban_remove))
        .routes(routes!(room_ban_get))
        .routes(routes!(room_ban_list))
}
