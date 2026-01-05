use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelReorder, ChannelType,
    MessageId, RatelimitPut, Room, RoomCreate, RoomMemberOrigin, RoomType, ThreadMemberPut, UserId,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::{
    types::{
        Channel, ChannelCreate, ChannelId, ChannelPatch, DbChannelCreate, DbChannelType,
        DbRoomCreate, MediaLinkType, MessageSync, MessageVerId, Permission, RoomId,
    },
    Error, ServerState,
};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};

use super::util::{Auth2, HeaderReason};
use crate::error::Result;

/// Room channel create
///
/// Create a channel in a room
#[utoipa::path(
    post,
    path = "/room/{room_id}/channel",
    params(("room_id", description = "Room id")),
    tags = [
        "channel",
        "badge.perm-opt.ChannelManage",
        "badge.perm-opt.ThreadCreatePublic",
        "badge.perm-opt.ThreadCreatePrivate",
    ],
    responses(
        (status = CREATED, body = Channel, description = "Create thread success"),
    )
)]
async fn channel_create_room(
    Path((room_id,)): Path<(RoomId,)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(mut json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    if json.ty.is_thread() {
        if let Some(parent_id) = json.parent_id {
            let parent_channel = s
                .services()
                .channels
                .get(parent_id, Some(auth.user.id))
                .await?;
            if json.auto_archive_duration.is_none() {
                json.auto_archive_duration = parent_channel.default_auto_archive_duration;
            }
        }
    }

    json.validate()?;
    let channel = s
        .services()
        .channels
        .create_channel(auth.user.id, Some(room_id), reason, json)
        .await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

// TODO: rename to /api/v1/user/@self/channel
// TODO: move to channels service
// TODO: unhardcode bitrate
/// Channel create dm
///
/// Create a dm or group dm thread (outside of a room)
#[utoipa::path(
    post,
    path = "/channel",
    tags = ["channel"],
    responses(
        (status = CREATED, body = Channel, description = "Create thread success"),
    )
)]
async fn channel_create_dm(
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Json(mut json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let data = s.data();
    match json.ty {
        ChannelType::Dm => {
            let Some(recipients) = &json.recipients else {
                return Err(Error::BadStatic("dm thread is missing recipients"));
            };
            if recipients.len() != 1 {
                return Err(Error::BadStatic(
                    "dm threads can only be with a single person",
                ));
            }
            let target_user_id = recipients.first().unwrap();
            let (thread, is_new) = srv.users.init_dm(auth.user.id, *target_user_id).await?;
            s.broadcast(MessageSync::ChannelCreate {
                channel: Box::new(thread.clone()),
            })?;
            if is_new {
                return Ok((StatusCode::CREATED, Json(thread)));
            } else {
                return Ok((StatusCode::OK, Json(thread)));
            }
        }
        ChannelType::Gdm => {
            let Some(recipients) = &mut json.recipients else {
                return Err(Error::BadStatic("gdm thread is missing recipients"));
            };
            recipients.push(auth.user.id);

            if recipients.len() as u32 > crate::consts::MAX_GDM_MEMBERS {
                return Err(Error::BadStatic("group dm has too many members"));
            }
        }
        _ => {
            return Err(Error::BadStatic(
                "can only create a dm/gdm thread outside of a room",
            ))
        }
    };

    if json.bitrate.is_some_and(|b| b > 393216) {
        return Err(Error::BadStatic("bitrate is too high"));
    }
    if !json.ty.has_voice() && json.bitrate.is_some() {
        return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
    }
    if !json.ty.has_voice() && json.user_limit.is_some() {
        return Err(Error::BadStatic(
            "cannot set user_limit for non voice thread",
        ));
    }

    if let Some(icon) = json.icon {
        if json.ty != ChannelType::Gdm {
            return Err(Error::BadStatic("only gdm threads can have icons"));
        }
        let media = data.media_select(icon).await?;
        if !matches!(
            media.inner.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
    }

    let channel_id = data
        .channel_create(DbChannelCreate {
            room_id: None,
            creator_id: auth.user.id,
            name: json.name.clone(),
            description: json.description.clone(),
            icon: json.icon.map(|i| *i),
            ty: DbChannelType::Gdm,
            nsfw: json.nsfw,
            bitrate: json.bitrate.map(|b| b as i32),
            user_limit: json.bitrate.map(|u| u as i32),
            parent_id: None,
            owner_id: Some(*auth.user.id),
            invitable: json.invitable,
            auto_archive_duration: json.auto_archive_duration.map(|d| d as i64),
            default_auto_archive_duration: json.default_auto_archive_duration.map(|d| d as i64),
            slowmode_thread: json.slowmode_thread.map(|d| d as i64),
            slowmode_message: json.slowmode_message.map(|d| d as i64),
            default_slowmode_message: json.default_slowmode_message.map(|d| d as i64),
            tags: json.tags,
            url: json.url,
        })
        .await?;

    if let Some(icon) = json.icon {
        data.media_link_create_exclusive(icon, *channel_id, MediaLinkType::IconThread)
            .await?;
    }

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    let mut members = vec![];

    if let Some(recipients) = &json.recipients {
        for id in recipients {
            data.thread_member_put(channel_id, *id, ThreadMemberPut {})
                .await?;
            let thread_member = data.thread_member_get(channel_id, *id).await?;
            members.push(thread_member);
        }
    }

    s.broadcast(MessageSync::ChannelCreate {
        channel: Box::new(thread.clone()),
    })?;
    for member in members {
        s.broadcast(MessageSync::ThreadMemberUpsert { member })?;
    }

    Ok((StatusCode::CREATED, Json(thread)))
}

/// Channel get
#[utoipa::path(
    get,
    path = "/channel/{channel_id}",
    params(("channel_id", description = "channel id")),
    tags = ["channel"],
    responses(
        (status = OK, body = Channel, description = "Get thread success"),
    )
)]
async fn channel_get(
    Path((channel_id,)): Path<(ChannelId,)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let channel = s
        .services()
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?;
    Ok((StatusCode::OK, Json(channel)))
}

#[derive(Deserialize, ToSchema, IntoParams)]
struct ChannelListQuery {
    parent_id: Option<ChannelId>,
}

/// Room channel list
#[utoipa::path(
    get,
    path = "/room/{room_id}/channel",
    params(
        ("room_id", description = "Room id"),
        ChannelListQuery,
        PaginationQuery<channelId>
    ),
    tags = ["channel"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "List room channels success"),
    )
)]
async fn channel_list(
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<ChannelListQuery>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let _perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    let mut res = data
        .channel_list(room_id, auth.user.id, pagination, q.parent_id)
        .await?;
    let srv = s.services();
    let ids: Vec<_> = res.items.iter().map(|t| t.id).collect();
    let channels = srv.channels.get_many(&ids, Some(auth.user.id)).await?;
    let mut channels_map: HashMap<_, _> = channels.into_iter().map(|c| (c.id, c)).collect();
    res.items = ids
        .into_iter()
        .filter_map(|id| channels_map.remove(&id))
        .collect();
    Ok(Json(res))
}

/// Room channel list removed
///
/// List removed threads in a room. Requires the `ChannelManage` permission.
#[utoipa::path(
    get,
    path = "/room/{room_id}/channel/removed",
    params(
        ("room_id", description = "Room id"),
        ChannelListQuery,
        PaginationQuery<channelId>
    ),
    tags = ["channel"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "List removed room threads success"),
    )
)]
async fn channel_list_removed(
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<ChannelListQuery>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ChannelManage)?;
    let mut res = data
        .channel_list_removed(room_id, auth.user.id, pagination, q.parent_id)
        .await?;
    let srv = s.services();
    let ids: Vec<_> = res.items.iter().map(|t| t.id).collect();
    let channels = srv.channels.get_many(&ids, Some(auth.user.id)).await?;
    let mut channels_map: HashMap<_, _> = channels.into_iter().map(|c| (c.id, c)).collect();
    res.items = ids
        .into_iter()
        .filter_map(|id| channels_map.remove(&id))
        .collect();
    Ok(Json(res))
}

/// Room channel reorder
///
/// Reorder the channels in a room. Requires the `ChannelManage` permission.
#[utoipa::path(
    patch,
    path = "/room/{room_id}/channel",
    params(("room_id", description = "Room id")),
    tags = ["channel", "badge.perm.ChannelManage"],
    responses(
        (status = OK, body = (), description = "Reorder channels success"),
    )
)]
async fn channel_reorder(
    Path((room_id,)): Path<(RoomId,)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ChannelReorder>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let _perms = srv.perms.for_room(auth.user.id, room_id).await?;

    let mut channels_old = HashMap::new();

    for channel in &json.channels {
        let channel_data = srv.channels.get(channel.id, None).await?;
        channels_old.insert(channel_data.id, channel_data);

        let perms_chan = srv.perms.for_channel(auth.user.id, channel.id).await?;
        perms_chan.ensure(Permission::ViewChannel)?;
        perms_chan.ensure(Permission::ChannelManage)?;

        if let Some(Some(parent_id)) = channel.parent_id {
            let perms_parent = srv.perms.for_channel(auth.user.id, parent_id).await?;
            perms_chan.ensure(Permission::ViewChannel)?;
            perms_parent.ensure(Permission::ChannelManage)?;

            let parent_data = srv.channels.get(parent_id, None).await?;
            if parent_data.ty != ChannelType::Category {
                return Err(Error::BadStatic(
                    "channels can only be children of category channels",
                ));
            }
        }
    }

    data.channel_reorder(json.clone()).await?;

    for chan in &json.channels {
        srv.channels.invalidate(chan.id).await;
        let chan_old = channels_old.get(&chan.id);
        let chan = srv.channels.get(chan.id, None).await?;
        if let Some(thread_old) = chan_old {
            if chan.parent_id == thread_old.parent_id && chan.position == thread_old.position {
                continue;
            }
        }
        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason,
        ty: AuditLogEntryType::ChannelReorder {
            channels: json.channels,
        },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Channel edit
#[utoipa::path(
    patch,
    path = "/channel/{channel_id}",
    params(
        ("channel_id", description = "channel id"),
    ),
    tags = ["channel", "badge.perm-opt.ChannelEdit", "badge.perm-opt.ThreadEdit"],
    responses(
        (status = OK, body = Channel, description = "edit message success"),
        (status = NOT_MODIFIED, body = Channel, description = "no change"),
    )
)]
async fn channel_update(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ChannelPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    if json.owner_id.is_some() {
        return Err(Error::BadStatic(
            "owner_id cannot be changed via this endpoint; use the transfer-ownership endpoint",
        ));
    }
    let chan = s
        .services()
        .channels
        .update(auth.user.id, channel_id, json.clone(), reason)
        .await?;

    if let Some(icon) = json.icon {
        s.data()
            .media_link_delete(*channel_id, MediaLinkType::IconThread)
            .await?;
        if let Some(icon) = icon {
            s.data()
                .media_link_create_exclusive(icon, *channel_id, MediaLinkType::IconThread)
                .await?;
        }
    }

    Ok(Json(chan))
}

// TODO: move to types/channel.rs?
#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckReq {
    /// The last read message id. Will be resolved from version_id if empty. (maybe remove later?)
    message_id: Option<MessageId>,

    /// The last read id in this channel.
    version_id: MessageVerId,

    /// The new mention count. Defaults to 0.
    mention_count: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckRes {
    /// The last read message id
    message_id: MessageId,

    /// The last read id in this channel. Currently unused, may be deprecated later?.
    version_id: MessageVerId,
}

/// Channel ack
///
/// Mark a channel as read (or unread).
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/ack",
    params(
        ("channel_id", description = "channel id"),
    ),
    tags = ["channel"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn channel_ack(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AckReq>,
) -> Result<Json<AckRes>> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let version_id = json.version_id;
    let message_id = if let Some(message_id) = json.message_id {
        message_id
    } else {
        data.message_version_get(channel_id, version_id, auth.user.id)
            .await?
            .id
    };
    data.unread_ack(
        auth.user.id,
        channel_id,
        message_id,
        version_id,
        json.mention_count,
    )
    .await?;
    srv.channels.invalidate_user(channel_id, auth.user.id).await;
    s.broadcast(MessageSync::ChannelAck {
        user_id: auth.user.id,
        channel_id,
        message_id,
        version_id,
    })?;
    Ok(Json(AckRes {
        message_id,
        version_id,
    }))
}

/// Channel remove
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/remove",
    params(("channel_id", description = "channel id")),
    tags = ["channel", "badge.perm.ThreadDelete"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn channel_remove(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if channel.ty.is_thread() {
        perms.ensure(Permission::ThreadManage)?;
    } else {
        perms.ensure(Permission::ChannelManage)?;
    }
    let chan_before = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if chan_before.deleted_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_delete(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    srv.voice.disconnect_everyone(channel_id)?;
    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan.ty,
                changes: Changes::new()
                    .change("deleted_at", &chan_before.deleted_at, &chan.deleted_at)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Channel restore
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/remove",
    params(("channel_id", description = "channel id")),
    tags = ["channel", "badge.perm.ThreadDelete"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn channel_restore(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if channel.ty.is_thread() {
        perms.ensure(Permission::ThreadManage)?;
    } else {
        perms.ensure(Permission::ChannelManage)?;
    }
    let chan_before = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if chan_before.deleted_at.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_undelete(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan.ty,
                changes: Changes::new()
                    .change("deleted_at", &chan_before.deleted_at, &chan.deleted_at)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Channel trigger typing indicator
///
/// Send a typing notification to a thread
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/typing",
    params(
        ("channel_id", description = "channel id"),
    ),
    tags = ["channel", "badge.perm.MessageCreate"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn channel_typing(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::MessageCreate)?;
    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }
    let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    srv.channels
        .typing_set(channel_id, auth.user.id, until)
        .await;
    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ChannelTyping {
            channel_id,
            user_id: auth.user.id,
            until: until.into(),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Channel upgrade
///
/// Convert a group dm thread into a full room. Only the gdm creator can upgrade the thread.
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/upgrade",
    params(("channel_id", description = "channel id")),
    tags = ["channel"],
    responses((status = OK, body = Room, description = "success")),
)]
async fn channel_upgrade(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if chan.ty != ChannelType::Gdm {
        return Err(Error::BadStatic("only group dms can be upgraded"));
    }

    if chan.owner_id != Some(auth.user.id) {
        return Err(Error::BadStatic("you are not the thread owner"));
    }

    if chan.room_id.is_some() {
        // NOTE: should this be a panic? gdms can't be in rooms anyways?
        return Err(Error::BadStatic("thread is already in a room"));
    }

    let room = srv
        .rooms
        .create(
            RoomCreate {
                name: chan.name.clone(),
                description: chan.description.clone(),
                icon: chan.icon,
                public: Some(false),
            },
            auth.user.id,
            DbRoomCreate {
                id: None,
                ty: RoomType::Default,
                welcome_channel_id: Some(channel_id),
            },
        )
        .await?;

    // TODO: run in a transaction
    // TODO: move to room create? i need to delete the thread icon before linking the room avatar
    if let Some(icon) = chan.icon {
        data.media_link_delete(*channel_id, MediaLinkType::IconThread)
            .await?;
        data.media_link_create_exclusive(icon, *room.id, MediaLinkType::AvatarRoom)
            .await?;
    }

    let mut members = vec![];
    let mut after: Option<Uuid> = None;
    loop {
        let page = data
            .thread_member_list(
                channel_id,
                PaginationQuery {
                    limit: Some(100),
                    from: after.map(|i| i.into()),
                    ..Default::default()
                },
            )
            .await?;

        if page.items.is_empty() {
            break;
        }

        after = Some(*page.items.last().unwrap().user_id);

        let page_len = page.items.len();
        members.extend(page.items);

        if page_len < 100 {
            break;
        }
    }

    data.channel_upgrade_gdm(channel_id, room.id).await?;

    for member in &members {
        data.room_member_put(
            room.id,
            member.user_id,
            Some(RoomMemberOrigin::GdmUpgrade),
            Default::default(),
        )
        .await?;
    }

    srv.channels.invalidate(channel_id).await;
    let upgraded_thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    s.broadcast(MessageSync::ChannelUpdate {
        channel: Box::new(upgraded_thread),
    })?;

    for member in members {
        let room_member = data.room_member_get(room.id, member.user_id).await?;
        s.broadcast_room(
            room.id,
            auth.user.id,
            MessageSync::RoomMemberUpsert {
                member: room_member,
            },
        )
        .await?;
    }

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: room.id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason,
        ty: AuditLogEntryType::ChannelUpdate {
            channel_id,
            channel_type: ChannelType::Text,
            changes: Changes::new()
                .change("type", &chan.ty, &ChannelType::Text)
                .change("room_id", &chan.room_id, &Some(room.id))
                .build(),
        },
    })
    .await?;

    Ok((StatusCode::OK, Json(room)))
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
struct TransferOwnership {
    owner_id: UserId,
}

/// Channel transfer ownership
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/transfer-ownership",
    params(("channel_id", description = "channel id")),
    tags = ["channel", "badge.sudo"],
    responses((status = OK, description = "success"))
)]
async fn channel_transfer_ownership(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<TransferOwnership>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_sudo()?;

    let srv = s.services();
    let target_user_id = json.owner_id;

    // ensure that target user is a thread member
    s.data()
        .thread_member_get(channel_id, target_user_id)
        .await?;

    let _perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    let thread_start = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if thread_start.owner_id != Some(auth.user.id) {
        return Err(Error::BadStatic("you aren't the thread owner"));
    }

    let thread = srv
        .channels
        .update(
            auth.user.id,
            channel_id,
            ChannelPatch {
                owner_id: Some(Some(target_user_id)),
                ..Default::default()
            },
            None,
        )
        .await?;

    let msg = MessageSync::ChannelUpdate {
        channel: Box::new(thread.clone()),
    };
    s.broadcast_channel(channel_id, auth.user.id, msg).await?;
    Ok(Json(thread))
}

/// Ratelimit delete
///
/// Immediately expires a slowmode ratelimit, allowing the target user to send a message again
/// Requires either ChannelManage, ThreadManage, or MemberTimeout
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/ratelimit/{user_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("user_id", description = "User id")
    ),
    tags = [
        "channel",
        "badge.perm-opt.ChannelManage",
        "badge.perm-opt.ThreadManage",
        "badge.perm-opt.MemberTimeout",
    ],
    responses(
        (status = NO_CONTENT, description = "Rate limit expired"),
    )
)]
async fn channel_ratelimit_delete(
    Path((channel_id, user_id)): Path<(ChannelId, UserId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(Error::MissingPermissions);
    }

    s.data()
        .channel_set_message_slowmode_expire_at(
            channel_id,
            user_id,
            (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into(),
        )
        .await?;
    s.data()
        .channel_set_thread_slowmode_expire_at(
            channel_id,
            user_id,
            (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into(),
        )
        .await?;

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason,
            ty: AuditLogEntryType::RatelimitUpdate {
                channel_id,
                user_id,
                slowmode_thread_expire_at: None,
                slowmode_message_expire_at: None,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::RatelimitUpdate {
            channel_id,
            user_id,
            slowmode_thread_expire_at: None,
            slowmode_message_expire_at: None,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Ratelimit update
///
/// Immediately creates a slowmode ratelimit
/// Requires either ChannelManage or ThreadManage, or MemberTimeout
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/ratelimit/{user_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("user_id", description = "User id")
    ),
    request_body = RatelimitPut,
    tags = [
        "channel",
        "badge.perm-opt.ChannelManage",
        "badge.perm-opt.ThreadManage",
        "badge.perm-opt.MemberTimeout",
    ],
    responses(
        (status = OK, description = "Rate limit updated"),
    )
)]
async fn channel_ratelimit_update(
    Path((channel_id, user_id)): Path<(ChannelId, UserId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RatelimitPut>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(Error::MissingPermissions);
    }

    let mut message_expire_at = None;
    let mut thread_expire_at = None;

    if let Some(expire_at_opt) = json.slowmode_message_expire_at {
        if let Some(expire_at) = expire_at_opt {
            s.data()
                .channel_set_message_slowmode_expire_at(channel_id, user_id, expire_at)
                .await?;
            message_expire_at = Some(expire_at);
        } else {
            s.data()
                .channel_set_message_slowmode_expire_at(
                    channel_id,
                    user_id,
                    (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1)))
                        .into(),
                )
                .await?;
            message_expire_at = None;
        }
    }

    if let Some(expire_at_opt) = json.slowmode_thread_expire_at {
        if let Some(expire_at) = expire_at_opt {
            s.data()
                .channel_set_thread_slowmode_expire_at(channel_id, user_id, expire_at)
                .await?;
            thread_expire_at = Some(expire_at);
        } else {
            s.data()
                .channel_set_thread_slowmode_expire_at(
                    channel_id,
                    user_id,
                    (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1)))
                        .into(),
                )
                .await?;
            thread_expire_at = None;
        }
    }

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason,
            ty: AuditLogEntryType::RatelimitUpdate {
                channel_id,
                user_id,
                slowmode_thread_expire_at: thread_expire_at,
                slowmode_message_expire_at: message_expire_at,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::RatelimitUpdate {
            channel_id,
            user_id,
            slowmode_thread_expire_at: thread_expire_at,
            slowmode_message_expire_at: message_expire_at,
        },
    )
    .await?;

    Ok(StatusCode::OK)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(channel_create_room))
        .routes(routes!(channel_create_dm))
        .routes(routes!(channel_get))
        .routes(routes!(channel_list))
        .routes(routes!(channel_list_removed))
        .routes(routes!(channel_reorder))
        .routes(routes!(channel_update))
        .routes(routes!(channel_ack))
        .routes(routes!(channel_remove))
        .routes(routes!(channel_restore))
        .routes(routes!(channel_typing))
        .routes(routes!(channel_upgrade))
        .routes(routes!(channel_transfer_ownership))
        .routes(routes!(channel_ratelimit_update))
        .routes(routes!(channel_ratelimit_delete))
}
