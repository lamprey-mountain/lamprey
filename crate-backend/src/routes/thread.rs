use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Changes;
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Channel, ChannelCreate, ChannelId,
    ChannelMemberSearch, ChannelMemberSearchResponse, ChannelType, Mentions, MentionsUser, Message,
    MessageId, MessageMember, MessageSync, MessageThreadCreated, MessageType, PaginationQuery,
    PaginationResponse, Permission, RoomId, ThreadMember, ThreadMemberPut, ThreadMembership,
    UserId, SERVER_ROOM_ID,
};
use http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::{DbChannelCreate, DbChannelType, DbMessageCreate, UserIdReq};
use crate::ServerState;

use super::util::{Auth2, HeaderReason};
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
async fn thread_member_list(
    Path(thread_id): Path<ChannelId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, thread_id)
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
async fn thread_member_get(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, thread_id)
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
async fn thread_member_add(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadMemberPut>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let thread = srv.channels.get(thread_id, Some(auth.user.id)).await?;
    if target_user_id != auth.user.id {
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
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    if thread.ty == ChannelType::Gdm {
        let is_joining = d
            .thread_member_get(thread_id, target_user_id)
            .await
            .map(|m| m.membership != ThreadMembership::Join)
            .unwrap_or(true);

        if is_joining {
            let count = d.thread_member_list_all(thread_id).await?.len() as u32;
            if count >= crate::consts::MAX_GDM_MEMBERS {
                return Err(Error::BadStatic("group dm is full"));
            }
        }
    }

    let start = d.thread_member_get(thread_id, target_user_id).await.ok();
    d.thread_member_put(thread_id, target_user_id, ThreadMemberPut {})
        .await?;
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    if start.is_some_and(|s| s == res) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    if target_user_id != auth.user.id {
        let message_id = d
            .message_create(crate::types::DbMessageCreate {
                channel_id: thread_id,
                attachment_ids: vec![],
                author_id: auth.user.id,
                embeds: vec![],
                message_type: MessageType::MemberAdd(MessageMember { target_user_id }),
                edited_at: None,
                created_at: None,
                mentions: Mentions {
                    users: vec![MentionsUser {
                        id: target_user_id,
                        // data serialization code ignores resolved_name
                        resolved_name: "(this should be ignored)".to_owned(),
                    }],
                    ..Default::default()
                },
            })
            .await?;
        let message = srv
            .messages
            .get(thread_id, message_id, auth.user.id)
            .await?;
        srv.channels.invalidate(thread_id).await; // message count
        s.broadcast_channel(
            thread_id,
            auth.user.id,
            MessageSync::MessageCreate {
                message: message.clone(),
            },
        )
        .await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth.user.id,
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
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
        auth.user.id,
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
async fn thread_member_delete(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
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
    let perms = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    if target_user_id != auth.user.id {
        perms.ensure(Permission::MemberKick)?;
    }

    let thread = srv.channels.get(thread_id, Some(auth.user.id)).await?;
    if !thread.ty.has_members() {
        return Err(Error::BadStatic("cannot edit thread member list"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
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

    if target_user_id != auth.user.id {
        let message_id = d
            .message_create(crate::types::DbMessageCreate {
                channel_id: thread_id,
                attachment_ids: vec![],
                author_id: auth.user.id,
                embeds: vec![],
                message_type: MessageType::MemberRemove(MessageMember { target_user_id }),
                edited_at: None,
                created_at: None,
                mentions: Default::default(),
            })
            .await?;
        let message = srv
            .messages
            .get(thread_id, message_id, auth.user.id)
            .await?;
        srv.channels.invalidate(thread_id).await; // message count
        s.broadcast_channel(
            thread_id,
            auth.user.id,
            MessageSync::MessageCreate {
                message: message.clone(),
            },
        )
        .await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth.user.id,
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
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
        auth.user.id,
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
async fn thread_list(
    Path(channel_id): Path<ChannelId>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let include_all = perms.has(Permission::ThreadManage);
    let mut res = data
        .thread_list_active(auth.user.id, pagination, channel_id, include_all)
        .await?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    res.items = srv
        .channels
        .get_many(&channel_ids, Some(auth.user.id))
        .await?;
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
async fn thread_list_archived(
    Path(channel_id): Path<ChannelId>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let include_all = perms.has(Permission::ThreadManage);
    let mut res = data
        .thread_list_archived(auth.user.id, pagination, channel_id, include_all)
        .await?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    res.items = srv
        .channels
        .get_many(&channel_ids, Some(auth.user.id))
        .await?;
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
async fn thread_list_removed(
    Path(channel_id): Path<ChannelId>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ThreadManage)?;

    let mut res = data
        .thread_list_removed(auth.user.id, pagination, channel_id, true)
        .await?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    res.items = srv
        .channels
        .get_many(&channel_ids, Some(auth.user.id))
        .await?;
    Ok(Json(res))
}

/// Thread list atom/rss (TODO)
///
/// Get an atom or rss feed of threads for this channel
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/thread.atom",
    params(
        ("channel_id", description = "Channel id"),
        PaginationQuery<ChannelId>
    ),
    tags = ["thread"],
)]
async fn thread_list_atom(
    Path(_channel_id): Path<ChannelId>,
    Query(_pagination): Query<PaginationQuery<ChannelId>>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
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
async fn thread_create(
    Path(parent_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(mut json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    if !matches!(
        json.ty,
        ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::ThreadForum2
    ) {
        return Err(Error::BadStatic("invalid thread type"));
    }

    let parent_channel = s
        .services()
        .channels
        .get(parent_id, Some(auth.user.id))
        .await?;
    let room_id = parent_channel.room_id;

    if json.auto_archive_duration.is_none() {
        json.auto_archive_duration = parent_channel.default_auto_archive_duration;
    }

    json.parent_id = Some(parent_id);
    json.validate()?;

    let channel = s
        .services()
        .channels
        .create_channel(auth.user.id, room_id, reason, json)
        .await?;

    Ok((StatusCode::CREATED, Json(channel)))
}

/// Thread create from message
///
/// Starts a new thread from a message. Requires the channel the message was
/// sent in to be threadable, ie. Text, Dm, Gdm. Forums will not work as threads
/// can't be created inside of other threads.
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/message/{message_id}/thread",
    params(
        ("channel_id", description = "Parent channel id"),
        ("message_id", description = "Source message id")
    ),
    request_body = ChannelCreate,
    tags = [
        "thread",
        "badge.perm.ThreadCreatePublic",
    ],
    responses(
        (status = CREATED, body = Channel, description = "Create thread success"),
        (status = CONFLICT, description = "A thread for this message already exists"),
    )
)]
async fn thread_create_from_message(
    Path((parent_channel_id, source_message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(mut json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    let srv = s.services();
    let data = s.data();

    // 1. Check permissions
    let perms = srv
        .perms
        .for_channel(auth.user.id, parent_channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::ThreadCreatePublic)?;

    // 2. Check if channel is threadable
    let parent_channel = srv
        .channels
        .get(parent_channel_id, Some(auth.user.id))
        .await?;
    if !parent_channel.ty.has_public_threads() && !parent_channel.ty.has_forum2_threads() {
        return Err(Error::BadStatic(
            "Cannot create a thread in this channel type",
        ));
    }

    // 3. Check if message exists and doesn't have a thread already
    let source_message = srv
        .messages
        .get(parent_channel_id, source_message_id, auth.user.id)
        .await?;
    if !source_message.latest_version.message_type.is_threadable() {
        return Err(Error::BadStatic(
            "Cannot create a thread from this message type",
        ));
    }

    let thread_id: ChannelId = (*source_message_id).into();
    if data.channel_get(thread_id).await.is_ok() {
        return Err(Error::Conflict);
    }

    // 4. Create the thread
    let room_id = parent_channel.room_id;

    // 5. Set auto_archive_duration to parent's default if not provided
    if json.auto_archive_duration.is_none() {
        json.auto_archive_duration = parent_channel.default_auto_archive_duration;
    }

    json.parent_id = Some(parent_channel_id);
    json.validate()?;

    let create = DbChannelCreate {
        room_id: room_id.map(|id| id.into_inner()),
        creator_id: auth.user.id,
        name: json.name.clone(),
        description: json.description.clone(),
        url: json.url.clone(),
        ty: if parent_channel.ty.has_forum2_threads() {
            DbChannelType::ThreadForum2
        } else {
            DbChannelType::ThreadPublic
        },
        nsfw: json.nsfw,
        bitrate: json.bitrate.map(|b| b as i32),
        user_limit: json.user_limit.map(|u| u as i32),
        parent_id: json.parent_id.map(|i| *i),
        owner_id: None,
        icon: None,
        invitable: json.invitable,
        auto_archive_duration: json.auto_archive_duration.map(|i| i as i64),
        default_auto_archive_duration: json.default_auto_archive_duration.map(|i| i as i64),
        slowmode_thread: json.slowmode_thread.map(|d| d as i64),
        slowmode_message: json.slowmode_message.map(|d| d as i64),
        default_slowmode_message: json.default_slowmode_message.map(|d| d as i64),
        tags: json.tags,
    };

    data.channel_create_with_id(thread_id, create).await?;

    // 5. Add creator as a member
    data.thread_member_put(thread_id, auth.user.id, ThreadMemberPut::default())
        .await?;

    let channel = srv.channels.get(thread_id, Some(auth.user.id)).await?;
    s.broadcast_channel(
        parent_channel_id,
        auth.user.id,
        MessageSync::ChannelCreate {
            channel: Box::new(channel.clone()),
        },
    )
    .await?;

    // 6. Conditionally create system message in the original thread
    let four_hours_ago = time::OffsetDateTime::now_utc() - time::Duration::hours(4);
    if source_message.created_at.into_inner() < four_hours_ago {
        let system_message_id = data
            .message_create(DbMessageCreate {
                channel_id: parent_channel_id,
                attachment_ids: vec![],
                author_id: auth.user.id,
                embeds: vec![],
                message_type: MessageType::ThreadCreated(MessageThreadCreated {
                    source_message_id: Some(source_message_id),
                }),
                edited_at: None,
                created_at: None,
                mentions: Default::default(),
            })
            .await?;

        let system_message = srv
            .messages
            .get(parent_channel_id, system_message_id, auth.user.id)
            .await?;
        s.broadcast_channel(
            parent_channel_id,
            auth.user.id,
            MessageSync::MessageCreate {
                message: system_message,
            },
        )
        .await?;
    }

    if let Some(room_id) = room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason,
            ty: AuditLogEntryType::ChannelCreate {
                channel_id: thread_id,
                channel_type: channel.ty,
                changes: Changes::new()
                    .add("name", &channel.name)
                    .add("description", &channel.description)
                    .add("nsfw", &channel.nsfw)
                    .add("user_limit", &channel.user_limit)
                    .add("bitrate", &channel.bitrate)
                    .add("type", &channel.ty)
                    .add("parent_id", &channel.parent_id)
                    .build(),
            },
        })
        .await?;
    }

    Ok((StatusCode::CREATED, Json(channel)))
}

#[derive(Serialize, ToSchema)]
pub struct ThreadListRoom {
    /// threads in this room
    pub threads: Vec<Channel>,

    /// only your own thread member objects
    pub thread_members: Vec<ThreadMember>,
}

/// Thread list room
///
/// List all active threads in a room
#[utoipa::path(
    get,
    path = "/room/{room_id}/thread",
    params(("room_id", description = "Room id")),
    tags = ["thread"],
    responses((status = OK, body = ThreadListRoom, description = "List room threads success")),
)]
async fn thread_list_room(
    Path(room_id): Path<RoomId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let user_id = auth.user.id;

    // just check if the user is a room member
    let _perms = srv.perms.for_room(user_id, room_id).await?;

    let all_threads = data.thread_all_active_room(room_id).await?;

    let thread_ids: Vec<_> = all_threads.iter().map(|t| t.id).collect();
    let thread_members = data.thread_member_bulk_fetch(user_id, &thread_ids).await?;
    let thread_members: HashMap<_, _> = thread_members.into_iter().collect();

    let mut filtered_thread_ids = vec![];
    for t in all_threads {
        // this *should* be cached and not too horrible performance wise?
        let perms = srv.perms.for_channel(auth.user.id, t.id).await?;
        let can_view = if t.ty == ChannelType::ThreadPublic {
            perms.has(Permission::ViewChannel)
        } else {
            perms.has(Permission::ThreadManage) || thread_members.get(&t.id).is_some()
        };
        if can_view {
            filtered_thread_ids.push(t.id);
        }
    }

    let threads = srv
        .channels
        .get_many(&filtered_thread_ids, Some(user_id))
        .await?;

    Ok(Json(ThreadListRoom {
        threads,
        thread_members: thread_members.into_values().collect(),
    }))
}

/// Thread activity
///
/// List activity in this thread
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/activity",
    params(
        ("channel_id", description = "Channel id"),
        PaginationQuery<MessageId>
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List activity success"),
    )
)]
async fn thread_activity(
    Path((channel_id,)): Path<(ChannelId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let res = srv
        .messages
        .list_activity(channel_id, auth.user.id, q)
        .await?;

    Ok(Json(res))
}

/// Channel member search
///
/// If this is a thread, search thread members. Otherwise, search all room members who can view this thread.
///
/// For mention autocomplete
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/member/search",
    params(
        ChannelMemberSearch,
        ("channel_id" = ChannelId, description = "Channel id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = ChannelMemberSearchResponse, description = "success"),
    )
)]
async fn channel_member_search(
    Path(channel_id): Path<ChannelId>,
    Query(search): Query<ChannelMemberSearch>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let _d = s.data();
    let srv = s.services();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, None).await?;

    // extra permission check to prevent returning the entire list of registered users
    if chan.room_id == Some(SERVER_ROOM_ID) {
        perms.ensure(Permission::ServerOversee)?;
    }

    let _limit = search.limit.unwrap_or(10).min(100);
    // let room_members = d.room_member_search(room_id, search.query, limit).await?;
    // let user_ids: Vec<UserId> = room_members.iter().map(|m| m.user_id).collect();
    // let users = srv.users.get_many(&user_ids).await?;

    // Ok(Json(ChannelMemberSearchResponse {
    //     room_members,
    //     thread_members,
    //     users,
    // }))

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_create))
        .routes(routes!(thread_create_from_message))
        .routes(routes!(thread_member_list))
        .routes(routes!(thread_member_get))
        .routes(routes!(thread_member_add))
        .routes(routes!(thread_member_delete))
        .routes(routes!(thread_list))
        .routes(routes!(thread_list_archived))
        .routes(routes!(thread_list_removed))
        .routes(routes!(thread_list_atom))
        .routes(routes!(thread_list_room))
        .routes(routes!(thread_activity))
        .routes(routes!(channel_member_search))
}
