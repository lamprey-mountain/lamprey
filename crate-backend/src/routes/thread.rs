use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Channel, ChannelCreate, ChannelId,
    ChannelType, MessageMember, MessageSync, MessageType, PaginationQuery, PaginationResponse,
    Permission, ThreadMember, ThreadMemberPut, ThreadMembership, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};

/// Thread member list
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/member",
    params(
        PaginationQuery<UserId>,
        ("thread_id" = ChannelId, description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<ThreadMember>, description = "success"),
    )
)]
pub async fn thread_member_list(
    Path(thread_id): Path<ChannelId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, thread_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = d.thread_member_list(thread_id, paginate).await?;
    Ok(Json(res))
}

/// Thread member get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/member/{user_id}",
    params(
        ("thread_id" = ChannelId, description = "Thread id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = ThreadMember, description = "success"),
    )
)]
pub async fn thread_member_get(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, thread_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    // TODO: return `Ban`s
    if !matches!(res.membership, ThreadMembership::Join { .. }) {
        Err(Error::NotFound)
    } else {
        Ok(Json(res))
    }
}

/// Thread member add
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/member/{user_id}",
    params(
        ("thread_id" = ChannelId, description = "Thread id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["thread", "badge.perm-opt.MemberKick"],
    responses(
        (status = OK, body = ThreadMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
pub async fn thread_member_add(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadMemberPut>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let thread = srv.channels.get(thread_id, Some(auth_user.id)).await?;
    if target_user_id != auth_user.id {
        if !thread.invitable {
            perms.ensure(Permission::MemberKick)?;
        }
    }
    if !thread.ty.has_members() {
        return Err(Error::BadStatic("cannot edit thread member list"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    let start = d.thread_member_get(thread_id, target_user_id).await.ok();
    d.thread_member_put(thread_id, target_user_id, ThreadMemberPut {})
        .await?;
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    if start.is_some_and(|s| s == res) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    if target_user_id != auth_user.id {
        let message_id = d
            .message_create(crate::types::DbMessageCreate {
                channel_id: thread_id,
                attachment_ids: vec![],
                author_id: auth_user.id,
                embeds: vec![],
                message_type: MessageType::MemberAdd(MessageMember { target_user_id }),
                edited_at: None,
                created_at: None,
                mentions: Default::default(),
            })
            .await?;
        let message = d.message_get(thread_id, message_id, auth_user.id).await?;
        srv.channels.invalidate(thread_id).await; // message count
        s.broadcast_channel(
            thread_id,
            auth_user.id,
            MessageSync::MessageCreate {
                message: message.clone(),
            },
        )
        .await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason,
                ty: AuditLogEntryType::ThreadMemberAdd {
                    thread_id,
                    user_id: target_user_id,
                },
            })
            .await?;
        }
    }

    s.broadcast_channel(
        thread_id,
        auth_user.id,
        MessageSync::ThreadMemberUpsert {
            member: res.clone(),
        },
    )
    .await?;
    Ok(Json(res).into_response())
}

/// Thread member delete
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/member/{user_id}",
    params(
        ("thread_id" = ChannelId, description = "Thread id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["thread", "badge.perm-opt.MemberKick"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn thread_member_delete(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    if target_user_id != auth_user.id {
        perms.ensure(Permission::MemberKick)?;
    }

    let thread = srv.channels.get(thread_id, Some(auth_user.id)).await?;
    if !thread.ty.has_members() {
        return Err(Error::BadStatic("cannot edit thread member list"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    let start = d.thread_member_get(thread_id, target_user_id).await?;
    if !matches!(start.membership, ThreadMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    d.thread_member_set_membership(thread_id, target_user_id, ThreadMembership::Leave {})
        .await?;
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    if start == res {
        return Ok(StatusCode::NOT_MODIFIED);
    }

    s.services()
        .perms
        .invalidate_thread(target_user_id, thread_id);

    if target_user_id != auth_user.id {
        let message_id = d
            .message_create(crate::types::DbMessageCreate {
                channel_id: thread_id,
                attachment_ids: vec![],
                author_id: auth_user.id,
                embeds: vec![],
                message_type: MessageType::MemberRemove(MessageMember { target_user_id }),
                edited_at: None,
                created_at: None,
                mentions: Default::default(),
            })
            .await?;
        let message = d.message_get(thread_id, message_id, auth_user.id).await?;
        srv.channels.invalidate(thread_id).await; // message count
        s.broadcast_channel(
            thread_id,
            auth_user.id,
            MessageSync::MessageCreate {
                message: message.clone(),
            },
        )
        .await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason,
                ty: AuditLogEntryType::ThreadMemberRemove {
                    thread_id,
                    user_id: target_user_id,
                },
            })
            .await?;
        }
    }

    s.broadcast_channel(
        thread_id,
        auth_user.id,
        MessageSync::ThreadMemberUpsert { member: res },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Thread list
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/thread",
    params(
        ("channel_id", description = "Channel id"),
        PaginationQuery<ChannelId>
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "List channel threads success"),
    )
)]
pub async fn thread_list(
    Path(channel_id): Path<ChannelId>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let include_all = perms.has(Permission::ThreadManage);
    let mut res = data
        .thread_list_active(auth_user.id, pagination, channel_id, include_all)
        .await?;

    let mut channels = vec![];
    for c in &res.items {
        channels.push(srv.channels.get(c.id, Some(auth_user.id)).await?);
    }
    res.items = channels;
    Ok(Json(res))
}

/// Thread list archived
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/thread/archived",
    params(
        ("channel_id", description = "Channel id"),
        PaginationQuery<ChannelId>
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "List channel archived threads success"),
    )
)]
pub async fn thread_list_archived(
    Path(channel_id): Path<ChannelId>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let include_all = perms.has(Permission::ThreadManage);
    let mut res = data
        .thread_list_archived(auth_user.id, pagination, channel_id, include_all)
        .await?;

    let mut channels = vec![];
    for c in &res.items {
        channels.push(srv.channels.get(c.id, Some(auth_user.id)).await?);
    }
    res.items = channels;
    Ok(Json(res))
}

/// Thread list removed
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/thread/removed",
    params(
        ("channel_id", description = "Channel id"),
        PaginationQuery<ChannelId>
    ),
    tags = ["thread", "badge.perm.ThreadManage"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "List channel removed threads success"),
    )
)]
pub async fn thread_list_removed(
    Path(channel_id): Path<ChannelId>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ThreadManage)?;

    let mut res = data
        .thread_list_removed(auth_user.id, pagination, channel_id, true)
        .await?;

    let mut channels = vec![];
    for c in &res.items {
        channels.push(srv.channels.get(c.id, Some(auth_user.id)).await?);
    }
    res.items = channels;
    Ok(Json(res))
}

/// Thread create
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/thread",
    params(("channel_id", description = "Parent channel id")),
    tags = [
        "thread",
        "badge.perm-opt.ThreadCreatePublic",
        "badge.perm-opt.ThreadCreatePrivate",
    ],
    responses(
        (status = CREATED, body = Channel, description = "Create thread success"),
    )
)]
pub async fn thread_create(
    Path(parent_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(mut json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;

    if !matches!(
        json.ty,
        ChannelType::ThreadPublic | ChannelType::ThreadPrivate
    ) {
        return Err(Error::BadStatic("invalid thread type"));
    }

    let parent_channel = s
        .services()
        .channels
        .get(parent_id, Some(auth_user.id))
        .await?;
    let room_id = parent_channel
        .room_id
        .ok_or(Error::BadStatic("Parent channel not in a room"))?;

    json.parent_id = Some(parent_id);

    let channel = s
        .services()
        .channels
        .create_channel(auth_user.id, room_id, reason, json)
        .await?;

    Ok((StatusCode::CREATED, Json(channel)))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_create))
        .routes(routes!(thread_member_list))
        .routes(routes!(thread_member_get))
        .routes(routes!(thread_member_add))
        .routes(routes!(thread_member_delete))
        .routes(routes!(thread_list))
        .routes(routes!(thread_list_archived))
        .routes(routes!(thread_list_removed))
}
