use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Diff;
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, Permission, RoomId, RoomMember,
    RoomMemberPatch, RoomMemberPut, RoomMembership, UserId,
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
    if !matches!(
        res.membership,
        RoomMembership::Join { .. } | RoomMembership::Ban { .. }
    ) {
        Err(Error::NotFound)
    } else {
        Ok(Json(res))
    }
}

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
    d.room_member_put(
        room_id,
        target_user_id,
        RoomMembership::Join {
            override_name: json.override_name,
            override_description: json.override_description,
            roles: vec![],
        },
    )
    .await?;
    d.role_apply_default(room_id, target_user_id).await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
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
    d.room_member_patch(room_id, target_user_id, json).await?;
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
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
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberKick)?;
    }
    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave {})
        .await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
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
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure(Permission::MemberBan)?;

    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Ban {})
        .await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
        MessageSync::RoomMemberUpsert { member: res },
    )
    .await?;
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

    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave {})
        .await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
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
    let res = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(res.membership, RoomMembership::Ban { .. }) {
        Err(Error::NotFound)
    } else {
        Ok(Json(res))
    }
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
    let res = d.room_member_list(room_id, paginate).await?;
    let res = PaginationResponse {
        items: res
            .items
            .into_iter()
            .filter(|m| matches!(m.membership, RoomMembership::Ban { .. }))
            .collect(),
        has_more: res.has_more,
        total: 0, // FIXME
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
        .routes(routes!(room_ban_create))
        .routes(routes!(room_ban_remove))
        .routes(routes!(room_ban_get))
        .routes(routes!(room_ban_list))
}
