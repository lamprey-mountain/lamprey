use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    util::Changes, voice::SfuCommand, AuditLogEntry, AuditLogEntryId, AuditLogEntryType,
    ChannelReorder, ChannelType, MessageId, Room, RoomCreate, RoomMemberOrigin, RoomType,
    ThreadMemberPut, UserId,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::{
    routes::util::AuthSudo,
    types::{
        Channel, ChannelCreate, ChannelId, ChannelPatch, DbChannelCreate, DbChannelType,
        DbRoomCreate, MessageSync, MessageVerId, Permission, RoomId,
    },
    Error, ServerState,
};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};

use super::util::{Auth, HeaderReason};
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let data = s.data();
    let perms = if let Some(parent_id) = json.parent_id {
        srv.perms.for_channel(auth_user.id, parent_id).await?
    } else {
        srv.perms.for_room(auth_user.id, room_id).await?
    };
    perms.ensure(Permission::ViewChannel)?;
    match json.ty {
        ChannelType::Text | ChannelType::Forum | ChannelType::Voice | ChannelType::Category => {
            perms.ensure(Permission::ChannelManage)?;
        }
        ChannelType::ThreadPublic => {
            let parent_id = json
                .parent_id
                .ok_or(Error::BadStatic("threads must have a parent channel"))?;
            let parent = srv.channels.get(parent_id, Some(auth_user.id)).await?;
            if !matches!(parent.ty, ChannelType::Text | ChannelType::Forum) {
                return Err(Error::BadStatic(
                    "threads can only be created in text or forum channels",
                ));
            }
            perms.ensure(Permission::ThreadCreatePublic)?;
        }
        ChannelType::ThreadPrivate => {
            let parent_id = json
                .parent_id
                .ok_or(Error::BadStatic("threads must have a parent channel"))?;
            let parent = srv.channels.get(parent_id, Some(auth_user.id)).await?;
            if !matches!(parent.ty, ChannelType::Text | ChannelType::Forum) {
                return Err(Error::BadStatic(
                    "threads can only be created in text or forum channels",
                ));
            }
            perms.ensure(Permission::ThreadCreatePrivate)?;
        }
        ChannelType::Calendar => return Err(Error::BadStatic("not yet implemented")),
        // ThreadType::{ThreadPublic, ThreadPrivate} => require a parent_id, require parent to either be Text or Forum
        ChannelType::Dm | ChannelType::Gdm => {
            return Err(Error::BadStatic(
                "can't create a direct message thread in a room",
            ))
        }
    };
    if json.bitrate.is_some_and(|b| b > 393216) {
        return Err(Error::BadStatic("bitrate is too high"));
    }
    if json.ty != ChannelType::Voice && json.bitrate.is_some() {
        return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
    }
    if json.ty != ChannelType::Voice && json.user_limit.is_some() {
        return Err(Error::BadStatic(
            "cannot set user_limit for non voice thread",
        ));
    }
    let channel_id = data
        .channel_create(DbChannelCreate {
            room_id: Some(room_id.into_inner()),
            creator_id: auth_user.id,
            name: json.name.clone(),
            description: json.description.clone(),
            ty: match json.ty {
                ChannelType::Text => DbChannelType::Text,
                ChannelType::Forum => DbChannelType::Forum,
                ChannelType::Voice => DbChannelType::Voice,
                ChannelType::Category => DbChannelType::Category,
                ChannelType::ThreadPublic => DbChannelType::ThreadPublic,
                ChannelType::ThreadPrivate => DbChannelType::ThreadPrivate,
                ChannelType::Calendar => return Err(Error::BadStatic("not yet implemented")),
                ChannelType::Dm | ChannelType::Gdm => {
                    // this should be unreachable due to the check above
                    warn!("unreachable: dm/gdm thread creation in room");
                    return Err(Error::BadStatic(
                        "can't create a direct message thread in a room",
                    ));
                }
            },
            nsfw: json.nsfw,
            bitrate: json.bitrate.map(|b| b as i32),
            user_limit: json.user_limit.map(|u| u as i32),
            parent_id: json.parent_id.map(|i| *i),
            owner_id: None,
            icon: None,
        })
        .await?;

    data.thread_member_put(channel_id, auth_user.id, ThreadMemberPut {})
        .await?;
    let thread_member = data.thread_member_get(channel_id, auth_user.id).await?;

    let channel = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::ChannelCreate {
            channel_id,
            channel_type: channel.ty,
            changes: Changes::new()
                .add("name", &channel.name)
                .add("description", &channel.description)
                .add("nsfw", &channel.nsfw)
                .add("user_limit", &channel.user_limit)
                .add("bitrate", &channel.bitrate)
                .add("type", &channel.ty)
                .build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user.id,
        MessageSync::ChannelCreate {
            channel: Box::new(channel.clone()),
        },
    )
    .await?;
    s.broadcast_channel(
        channel.id,
        auth_user.id,
        MessageSync::ThreadMemberUpsert {
            member: thread_member,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(channel)))
}

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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(mut json): Json<ChannelCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
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
            let (thread, is_new) = srv.users.init_dm(auth_user.id, *target_user_id).await?;
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
            recipients.push(auth_user.id);
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
    if json.ty != ChannelType::Voice && json.bitrate.is_some() {
        return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
    }
    if json.ty != ChannelType::Voice && json.user_limit.is_some() {
        return Err(Error::BadStatic(
            "cannot set user_limit for non voice thread",
        ));
    }

    if let Some(icon) = json.icon {
        if json.ty != ChannelType::Gdm {
            return Err(Error::BadStatic("only gdm threads can have icons"));
        }
        let (media, _) = data.media_select(icon).await?;
        if !matches!(
            media.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
    }

    let channel_id = data
        .channel_create(DbChannelCreate {
            room_id: None,
            creator_id: auth_user.id,
            name: json.name.clone(),
            description: json.description.clone(),
            icon: json.icon.map(|i| *i),
            ty: DbChannelType::Gdm,
            nsfw: json.nsfw,
            bitrate: json.bitrate.map(|b| b as i32),
            user_limit: json.bitrate.map(|u| u as i32),
            parent_id: None,
            owner_id: Some(*auth_user.id),
        })
        .await?;

    if let Some(icon) = json.icon {
        data.media_link_create_exclusive(
            icon,
            *channel_id,
            crate::types::MediaLinkType::IconThread,
        )
        .await?;
    }

    let thread = srv.channels.get(channel_id, Some(auth_user.id)).await?;
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let channel = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let _perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    let mut res = data
        .channel_list(room_id, auth_user.id, pagination, q.parent_id)
        .await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        // FIXME: dubious performance
        threads.push(srv.channels.get(t.id, Some(auth_user.id)).await?);
    }
    res.items = threads;
    Ok(Json(res))
}

/// Room channel list archived
#[utoipa::path(
    get,
    path = "/room/{room_id}/channel/archived",
    params(
        ("room_id", description = "Room id"),
        ChannelListQuery,
        PaginationQuery<channelId>
    ),
    tags = ["channel"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "List archived room channels success"),
    )
)]
async fn channel_list_archived(
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<ChannelListQuery>,
    Query(pagination): Query<PaginationQuery<ChannelId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let _perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    let mut res = data
        .channel_list_archived(room_id, auth_user.id, pagination, q.parent_id)
        .await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.channels.get(t.id, Some(auth_user.id)).await?);
    }
    res.items = threads;
    Ok(Json(res))
}

/// Room channel list removed
///
/// List removed threads in a room. Requires the `ThreadDelete` permission.
#[utoipa::path(
    get,
    path = "/room/{room_id}/thread/removed",
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::ChannelManage)?;
    let mut res = data
        .channel_list_removed(room_id, auth_user.id, pagination, q.parent_id)
        .await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.channels.get(t.id, Some(auth_user.id)).await?);
    }
    res.items = threads;
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ChannelReorder>,
) -> Result<()> {
    let data = s.data();
    let srv = s.services();
    let _perms = srv.perms.for_room(auth_user.id, room_id).await?;

    let mut channels_old = HashMap::new();

    for channel in &json.channels {
        let channel_data = srv.channels.get(channel.id, None).await?;
        channels_old.insert(channel_data.id, channel_data);

        let perms_chan = srv.perms.for_channel(auth_user.id, channel.id).await?;
        perms_chan.ensure(Permission::ViewChannel)?;
        perms_chan.ensure(Permission::ChannelManage)?;

        if let Some(Some(parent_id)) = channel.parent_id {
            let perms_parent = srv.perms.for_channel(auth_user.id, parent_id).await?;
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
            auth_user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::ChannelReorder {
            channels: json.channels,
        },
    })
    .await?;

    Ok(())
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ChannelPatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    if json.owner_id.is_some() {
        return Err(Error::BadStatic(
            "owner_id cannot be changed via this endpoint; use the transfer-ownership endpoint",
        ));
    }
    let chan = s
        .services()
        .channels
        .update(auth_user.id, channel_id, json.clone(), reason)
        .await?;

    if let Some(icon) = json.icon {
        s.data()
            .media_link_delete(*channel_id, crate::types::MediaLinkType::IconThread)
            .await?;
        if let Some(icon) = icon {
            s.data()
                .media_link_create_exclusive(
                    icon,
                    *channel_id,
                    crate::types::MediaLinkType::IconThread,
                )
                .await?;
        }
    }

    Ok(Json(chan))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckReq {
    /// The last read message id. Will be resolved from version_id if empty. (maybe remove later?)
    message_id: Option<MessageId>,

    /// The last read id in this channel.
    version_id: MessageVerId,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AckReq>,
) -> Result<Json<AckRes>> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let version_id = json.version_id;
    let message_id = if let Some(message_id) = json.message_id {
        message_id
    } else {
        data.message_version_get(channel_id, version_id, auth_user.id)
            .await?
            .id
    };
    data.unread_put(auth_user.id, channel_id, message_id, version_id)
        .await?;
    s.services()
        .channels
        .invalidate_user(channel_id, auth_user.id)
        .await;
    Ok(Json(AckRes {
        message_id,
        version_id,
    }))
}

/// Channel archive
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/archive",
    params(
        ("channel_id", description = "channel id"),
    ),
    tags = ["channel", "badge.perm-opt.ChannelManage", "badge.perm-opt.ThreadArchive"],
    responses(
        (status = OK, body = Channel, description = "success"),
        (status = NOT_MODIFIED, body = Channel, description = "didn't change anything"),
    )
)]
async fn channel_archive(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let chan_before = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    if auth_user.id != chan_before.creator_id {
        perms.ensure(Permission::ChannelManage)?;
    }
    if chan_before.deleted_at.is_some() {
        return Err(Error::BadStatic("channel is removed"));
    }
    if chan_before.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    if chan_before.archived_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }

    data.channel_archive(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    srv.users.disconnect_everyone_from_thread(channel_id)?;
    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan.ty,
                changes: Changes::new()
                    .change("archived_at", &chan_before.archived_at, &chan.archived_at)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(
            room_id,
            auth_user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan.clone()),
            },
        )
        .await?;
        s.sushi_sfu
            .send(SfuCommand::Thread {
                thread: chan.into(),
            })
            .unwrap();
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Channel unarchive
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/archive",
    params(
        ("channel_id", description = "channel id"),
    ),
    tags = ["channel", "badge.perm-opt.ThreadManage", "badge.perm-opt.ChannelManage"],
    responses(
        (status = OK, body = Channel, description = "success"),
        (status = NOT_MODIFIED, body = Channel, description = "didn't change anything"),
    )
)]
async fn channel_unarchive(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let chan_before = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    if auth_user.id != chan_before.creator_id {
        perms.ensure(Permission::ChannelManage)?;
    }
    if chan_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if chan_before.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    if chan_before.archived_at.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_unarchive(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan.ty,
                changes: Changes::new()
                    .change("archived_at", &chan_before.archived_at, &chan.archived_at)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(
            room_id,
            auth_user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
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
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ChannelManage)?;
    let chan_before = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if chan_before.deleted_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_delete(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    srv.users.disconnect_everyone_from_thread(channel_id)?;
    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
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
            auth_user.id,
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
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ChannelManage)?;
    let chan_before = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if chan_before.deleted_at.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_undelete(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
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
            auth_user.id,
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
    method(post),
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::MessageCreate)?;
    let thread = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    srv.channels
        .typing_set(channel_id, auth_user.id, until)
        .await;
    s.broadcast_channel(
        channel_id,
        auth_user.id,
        MessageSync::ChannelTyping {
            channel_id,
            user_id: auth_user.id,
            until: until.into(),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Channel lock
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/lock",
    params(("channel_id", description = "channel id")),
    tags = ["channel", "badge.perm.ThreadLock"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn channel_lock(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let chan_before = srv.channels.get(channel_id, None).await?;
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::ThreadLock)?;
    if chan_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if chan_before.locked {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_lock(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    srv.users.disconnect_everyone_from_thread(channel_id)?;
    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan.ty,
                changes: Changes::new()
                    .change("locked", &chan_before.locked, &chan.locked)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(
            room_id,
            auth_user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Channel unlock
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/lock",
    params(("channel_id", description = "channel id")),
    tags = ["channel", "badge.perm.ThreadLock"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn channel_unlock(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let chan_before = srv.channels.get(channel_id, None).await?;
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::ThreadLock)?;
    if chan_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if !chan_before.locked {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.channel_unlock(channel_id).await?;
    srv.channels.invalidate(channel_id).await;
    srv.users.disconnect_everyone_from_thread(channel_id)?;
    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan.ty,
                changes: Changes::new()
                    .change("locked", &chan_before.locked, &chan.locked)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(
            room_id,
            auth_user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }
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
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();

    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;

    if chan.ty != ChannelType::Gdm {
        return Err(Error::BadStatic("only group dms can be upgraded"));
    }

    if chan.owner_id != Some(auth_user.id) {
        return Err(Error::BadStatic("you are not the thread owner"));
    }

    if chan.room_id.is_some() {
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
            auth_user.id,
            DbRoomCreate {
                id: None,
                ty: RoomType::Default,
                welcome_channel_id: Some(channel_id),
            },
        )
        .await?;

    if let Some(icon) = chan.icon {
        data.media_link_delete(*channel_id, crate::types::MediaLinkType::IconThread)
            .await?;
        data.media_link_create_exclusive(icon, *room.id, crate::types::MediaLinkType::AvatarRoom)
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
    let upgraded_thread = srv.channels.get(channel_id, Some(auth_user.id)).await?;

    s.broadcast(MessageSync::ChannelUpdate {
        channel: Box::new(upgraded_thread),
    })?;

    for member in members {
        let room_member = data.room_member_get(room.id, member.user_id).await?;
        s.broadcast_room(
            room.id,
            auth_user.id,
            MessageSync::RoomMemberUpsert {
                member: room_member,
            },
        )
        .await?;
    }

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: room.id,
        user_id: auth_user.id,
        session_id: None,
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
    AuthSudo(auth_user): AuthSudo,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<TransferOwnership>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let target_user_id = json.owner_id;

    // ensure that target user is a thread member
    s.data()
        .thread_member_get(channel_id, target_user_id)
        .await?;

    let _perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    let thread_start = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if thread_start.owner_id != Some(auth_user.id) {
        return Err(Error::BadStatic("you aren't the thread owner"));
    }

    let thread = srv
        .channels
        .update(
            auth_user.id,
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
    s.broadcast_channel(channel_id, auth_user.id, msg).await?;
    Ok(Json(thread))
}

// TODO: add these routes
// thread_list GET /channel/{channel_id}/thread
// thread_list_archived GET /channel/{channel_id}/thread/archived
// thread_list_removed GET /channel/{channel_id}/thread/removed
// these will be thread_list, existing thread_list routes will be channel_list

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(channel_create_room))
        .routes(routes!(channel_create_dm))
        .routes(routes!(channel_get))
        .routes(routes!(channel_list))
        .routes(routes!(channel_list_archived))
        .routes(routes!(channel_list_removed))
        .routes(routes!(channel_reorder))
        .routes(routes!(channel_update))
        .routes(routes!(channel_ack))
        .routes(routes!(channel_archive))
        .routes(routes!(channel_unarchive))
        .routes(routes!(channel_remove))
        .routes(routes!(channel_restore))
        .routes(routes!(channel_typing))
        .routes(routes!(channel_lock))
        .routes(routes!(channel_unlock))
        .routes(routes!(channel_upgrade))
        .routes(routes!(channel_transfer_ownership))
}
