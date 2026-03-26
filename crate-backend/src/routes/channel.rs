use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::ack::AckRes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::{
    AuditLogEntryType, ChannelType, RelationshipType, RoomCreate, RoomMemberOrigin, RoomType,
    ThreadMemberPut,
};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;
use validator::Validate;

use crate::routes::util::{Auth, AuthRelaxed2};
use crate::routes2;
use crate::types::{
    ChannelPatch, DbChannelCreate, DbChannelType, DbRoomCreate, MediaLinkType, MessageSync,
    Permission,
};
use crate::{error::Result, Error, ServerState};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};

/// Channel create room
#[handler(routes::channel_create_room)]
async fn channel_create_room(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_create_room::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let mut json = req.channel;
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
        .create_channel(&auth, Some(req.room_id), json, req.idempotency_key)
        .await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

/// Channel create dm
#[handler(routes::channel_create_dm)]
async fn channel_create_dm(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    mut req: routes::channel_create_dm::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.channel.validate()?;
    let srv = s.services();
    let data = s.data();
    srv.perms
        .for_server(auth.user.id)
        .await?
        .ensure(Permission::DmCreate)?;
    match req.channel.ty {
        ChannelType::Dm => {
            let Some(recipients) = &req.channel.recipients else {
                return Err(ApiError::from_code(ErrorCode::DmThreadMissingRecipients).into());
            };
            if recipients.len() != 1 {
                return Err(ApiError::from_code(ErrorCode::DmThreadSinglePersonOnly).into());
            }
            let target_user_id = recipients.first().unwrap();
            let (thread, is_new) = srv
                .users
                .init_dm(auth.user.id, *target_user_id, false)
                .await?;
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
            let recipients = req.channel.recipients.clone().unwrap_or_default();
            let mut recipients = recipients;
            recipients.push(auth.user.id);

            if recipients.len() as u32 > crate::consts::MAX_GDM_MEMBERS {
                return Err(ApiError::from_code(ErrorCode::GdmTooManyMembers).into());
            }

            for recipient_id in recipients.iter().filter(|id| **id != auth.user.id) {
                let relationship = data
                    .user_relationship_get(auth.user.id, *recipient_id)
                    .await?;

                let are_friends =
                    relationship.is_some_and(|r| r.relation == Some(RelationshipType::Friend));

                if !are_friends {
                    return Err(ApiError::from_code(ErrorCode::GdmRequiresFriend).into());
                }
            }

            req.channel.recipients = Some(recipients);
        }
        _ => return Err(ApiError::from_code(ErrorCode::DmGdmOnlyOutsideRoom).into()),
    };

    let json = req.channel;
    if json.bitrate.is_some_and(|b| b > 393216) {
        return Err(ApiError::from_code(ErrorCode::BitrateTooHigh).into());
    }
    if !json.ty.has_voice() && json.bitrate.is_some() {
        return Err(ApiError::from_code(ErrorCode::CannotSetBitrateForNonVoiceThread).into());
    }
    if !json.ty.has_voice() && json.user_limit.is_some() {
        return Err(ApiError::from_code(ErrorCode::CannotSetUserLimitForNonVoiceThread).into());
    }

    if let Some(icon) = json.icon {
        json.ty.ensure_has_icon()?;
        let media = data.media_select(icon).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
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
            locked: false,
        })
        .await?;

    if let Some(icon) = json.icon {
        data.media_link_create_exclusive(icon, *channel_id, MediaLinkType::ChannelIcon)
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
    if !members.is_empty() {
        s.broadcast(MessageSync::ThreadMemberUpsert {
            room_id: thread.room_id,
            thread_id: thread.id,
            added: members,
            removed: vec![],
        })?;
    }

    Ok((StatusCode::CREATED, Json(thread)))
}

/// Channel get
#[handler(routes::channel_get)]
async fn channel_get(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;

    let user_id = auth.user.as_ref().map(|u| u.id);

    let perms = s
        .services()
        .perms
        .for_channel2(user_id, req.channel_id)
        .await?;
    perms.ensure(Permission::ChannelView)?;
    let channel = s.services().channels.get(req.channel_id, user_id).await?;
    Ok(Json(channel))
}

/// Room channel list
#[handler(routes::channel_list)]
async fn channel_list(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();

    let user_id = auth.user.as_ref().map(|u| u.id);

    let _perms = srv.perms.for_room2(user_id, req.room_id).await?;
    let channels = data.channel_list(req.room_id).await?;
    let ids: Vec<_> = channels.iter().map(|t| t.id).collect();
    let channels = srv.channels.get_many(&ids, user_id).await?;
    let mut channels_map: HashMap<_, _> = channels.into_iter().map(|c| (c.id, c)).collect();
    let channels: Vec<_> = ids
        .into_iter()
        .filter_map(|id| channels_map.remove(&id))
        .collect();
    let total = channels.len() as u64;
    Ok(Json(PaginationResponse {
        items: channels,
        total,
        has_more: false,
        cursor: None,
    }))
}

/// Room channel list removed
#[handler(routes::channel_list_removed)]
async fn channel_list_removed(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_list_removed::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::ChannelManage)?;
    let mut res = data
        .channel_list_removed(req.room_id, req.pagination, req.parent_id)
        .await?;

    let mut items = Vec::new();
    for item in res.items {
        if srv
            .perms
            .for_channel(auth.user.id, item.id)
            .await?
            .has(Permission::ChannelView)
        {
            items.push(item);
        }
    }
    res.items = items;

    let ids: Vec<_> = res.items.iter().map(|t| t.id).collect();
    let channels = srv.channels.get_many(&ids, Some(auth.user.id)).await?;
    let mut channels_map: HashMap<_, _> = channels.into_iter().map(|c| (c.id, c)).collect();
    res.items = ids
        .into_iter()
        .filter_map(|id| channels_map.remove(&id))
        .collect();
    Ok(Json(PaginationResponse {
        items: res.items,
        total: res.total,
        has_more: res.has_more,
        cursor: res.cursor,
    }))
}

/// Room channel reorder
#[handler(routes::channel_reorder)]
async fn channel_reorder(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_reorder::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();
    let _perms = srv.perms.for_room(auth.user.id, req.room_id).await?;

    let al = auth.audit_log(req.room_id);

    let mut channels_old = HashMap::new();

    for channel in &req.reorder.channels {
        let channel_data = srv.channels.get(channel.id, None).await?;
        channels_old.insert(channel_data.id, channel_data.clone());

        let perms_chan = srv.perms.for_channel(auth.user.id, channel.id).await?;
        perms_chan.ensure(Permission::ChannelView)?;
        perms_chan.ensure(Permission::ChannelManage)?;

        if let Some(Some(parent_id)) = channel.parent_id {
            let perms_parent = srv.perms.for_channel(auth.user.id, parent_id).await?;
            perms_chan.ensure(Permission::ChannelView)?;
            perms_parent.ensure(Permission::ChannelManage)?;

            let parent_data = srv.channels.get(parent_id, None).await?;
            if !channel_data.ty.can_be_in(Some(parent_data.ty)) {
                return Err(ApiError::from_code(ErrorCode::InvalidParentChannelType).into());
            }
        }
    }

    data.channel_reorder(req.reorder.clone()).await?;
    data.room_template_mark_dirty(req.room_id).await?;

    for chan in &req.reorder.channels {
        srv.channels.invalidate(chan.id).await;
        let chan_old = channels_old.get(&chan.id);
        let chan = srv.channels.get(chan.id, None).await?;
        if let Some(thread_old) = chan_old {
            if chan.parent_id == thread_old.parent_id && chan.position == thread_old.position {
                continue;
            }
        }
        s.broadcast_room(
            req.room_id,
            auth.user.id,
            MessageSync::ChannelUpdate {
                channel: Box::new(chan),
            },
        )
        .await?;
    }

    al.commit_success(AuditLogEntryType::ChannelReorder {
        channels: req.reorder.channels,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Channel update
#[handler(routes::channel_update)]
async fn channel_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_update::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.patch.validate()?;
    if req.patch.owner_id.is_some() {
        return Err(ApiError::from_code(ErrorCode::OwnerIdCannotBeChanged).into());
    }

    let srv = s.services();
    let chan_pre = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = chan_pre.room_id {
        let room_perms = srv.perms.for_room(auth.user.id, room_id).await?;
        room_perms.ensure_view()?;
    }

    let chan = srv
        .channels
        .update(&auth, req.channel_id, req.patch.clone())
        .await?;

    if let Some(icon) = req.patch.icon {
        s.data()
            .media_link_delete(*req.channel_id, MediaLinkType::ChannelIcon)
            .await?;
        if let Some(icon) = icon {
            s.data()
                .media_link_create_exclusive(icon, *req.channel_id, MediaLinkType::ChannelIcon)
                .await?;
        }
    }

    Ok(Json(chan))
}

/// Channel ack
#[handler(routes::channel_ack)]
async fn channel_ack(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_ack::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let version_id = req.ack.version_id;
    let message_id = if let Some(message_id) = req.ack.message_id {
        message_id
    } else {
        data.message_id_get_by_version(req.channel_id, version_id)
            .await?
    };
    data.unread_ack(
        auth.user.id,
        req.channel_id,
        message_id,
        version_id,
        Some(req.ack.mention_count),
    )
    .await?;
    srv.channels
        .invalidate_user(req.channel_id, auth.user.id)
        .await;
    s.broadcast(MessageSync::ChannelAck {
        user_id: auth.user.id,
        channel_id: req.channel_id,
        message_id,
        version_id,
    })?;
    Ok(Json(AckRes {
        message_id,
        version_id,
    }))
}

/// Channel remove
#[handler(routes::channel_remove)]
async fn channel_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_remove::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();

    let chan_before = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    if chan_before.is_thread() {
        perms.ensure(Permission::ThreadManage)?;
    } else {
        perms.ensure(Permission::ChannelManage)?;
    }

    if let Some(room_id) = chan_before.room_id {
        let al = auth.audit_log(room_id);
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_sudo {
            auth.ensure_sudo()?;
        }
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let chan_before = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
        if chan_before.is_removed() {
            return Ok(StatusCode::NO_CONTENT);
        }
        data.channel_delete(req.channel_id).await?;
        data.room_template_mark_dirty(room_id).await?;
        srv.channels.invalidate(req.channel_id).await;
        srv.voice.disconnect_everyone(req.channel_id).await?;
        let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

        al.commit_success(AuditLogEntryType::ChannelUpdate {
            channel_id: req.channel_id,
            channel_type: chan.ty,
            changes: Changes::new()
                .change("deleted_at", &chan_before.deleted_at, &chan.deleted_at)
                .build(),
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
    } else {
        let chan_before = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
        if chan_before.is_removed() {
            return Ok(StatusCode::NO_CONTENT);
        }
        data.channel_delete(req.channel_id).await?;
        srv.channels.invalidate(req.channel_id).await;
        srv.voice.disconnect_everyone(req.channel_id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Channel restore
#[handler(routes::channel_restore)]
async fn channel_restore(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_restore::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_sudo {
            auth.ensure_sudo()?;
        }
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
        if channel.is_thread() {
            perms.ensure(Permission::ThreadManage)?;
        } else {
            perms.ensure(Permission::ChannelManage)?;
        }
        let chan_before = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
        if !chan_before.is_removed() {
            return Ok(StatusCode::NO_CONTENT);
        }
        data.channel_undelete(req.channel_id).await?;
        data.room_template_mark_dirty(room_id).await?;
        srv.channels.invalidate(req.channel_id).await;
        let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

        al.commit_success(AuditLogEntryType::ChannelUpdate {
            channel_id: req.channel_id,
            channel_type: chan.ty,
            changes: Changes::new()
                .change("deleted_at", &chan_before.deleted_at, &chan.deleted_at)
                .build(),
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
    } else {
        let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
        if channel.is_thread() {
            perms.ensure(Permission::ThreadManage)?;
        } else {
            perms.ensure(Permission::ChannelManage)?;
        }
        let chan_before = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
        if !chan_before.is_removed() {
            return Ok(StatusCode::NO_CONTENT);
        }
        data.channel_undelete(req.channel_id).await?;
        srv.channels.invalidate(req.channel_id).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Channel typing
#[handler(routes::channel_typing)]
async fn channel_typing(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_typing::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    perms.ensure(Permission::MessageCreate)?;
    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    perms.ensure_unlocked()?;
    let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    srv.channels
        .typing_set(req.channel_id, auth.user.id, until)
        .await;
    let until_time: common::v1::types::util::Time = until
        .try_into()
        .expect("typing indicator time is always valid");
    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::ChannelTyping {
            channel_id: req.channel_id,
            user_id: auth.user.id,
            until: until_time,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Channel upgrade
#[handler(routes::channel_upgrade)]
async fn channel_upgrade(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_upgrade::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if chan.ty != ChannelType::Gdm {
        return Err(ApiError::from_code(ErrorCode::OnlyGdmCanUpgrade).into());
    }

    if chan.owner_id != Some(auth.user.id) {
        return Err(ApiError::from_code(ErrorCode::NotThreadOwner).into());
    }

    if chan.room_id.is_some() {
        return Err(ApiError::from_code(ErrorCode::ThreadAlreadyInRoom).into());
    }

    let room = srv
        .rooms
        .create(
            RoomCreate {
                name: chan.name.clone(),
                description: chan.description.clone(),
                icon: chan.icon,
                banner: None,
                public: Some(false),
            },
            &auth,
            DbRoomCreate {
                id: None,
                ty: RoomType::Default,
                welcome_channel_id: Some(req.channel_id),
            },
            None,
        )
        .await?;

    if let Some(icon) = chan.icon {
        data.media_link_delete(*req.channel_id, MediaLinkType::ChannelIcon)
            .await?;
        data.media_link_create_exclusive(icon, *room.id, MediaLinkType::RoomIcon)
            .await?;
    }

    let mut members = vec![];
    let mut after: Option<Uuid> = None;
    loop {
        let page = data
            .thread_member_list(
                req.channel_id,
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

    data.channel_upgrade_gdm(req.channel_id, room.id).await?;

    for member in &members {
        data.room_member_put(
            room.id,
            member.user_id,
            Some(RoomMemberOrigin::GdmUpgrade),
            Default::default(),
        )
        .await?;
    }

    srv.channels.invalidate(req.channel_id).await;
    let upgraded_thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    s.broadcast(MessageSync::ChannelUpdate {
        channel: Box::new(upgraded_thread),
    })?;

    for member in members {
        let room_member = data.room_member_get(room.id, member.user_id).await?;
        let user = srv.users.get(member.user_id, None).await?;
        s.broadcast_room(
            room.id,
            auth.user.id,
            MessageSync::RoomMemberCreate {
                member: room_member,
                user,
            },
        )
        .await?;
    }

    let al = auth.audit_log(room.id);
    al.commit_success(AuditLogEntryType::ChannelUpdate {
        channel_id: req.channel_id,
        channel_type: ChannelType::Text,
        changes: Changes::new()
            .change("type", &chan.ty, &ChannelType::Text)
            .change("room_id", &chan.room_id, &Some(room.id))
            .build(),
    })
    .await?;

    Ok(Json(room))
}

/// Channel transfer ownership
#[handler(routes::channel_transfer_ownership)]
async fn channel_transfer_ownership(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_transfer_ownership::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    auth.ensure_sudo()?;

    let srv = s.services();
    let target_user_id = req.owner_id;

    s.data()
        .thread_member_get(req.channel_id, target_user_id)
        .await?;

    let _perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    let thread_start = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if thread_start.owner_id != Some(auth.user.id) {
        return Err(ApiError::from_code(ErrorCode::NotThreadOwner).into());
    }

    let thread = srv
        .channels
        .update(
            &auth,
            req.channel_id,
            ChannelPatch {
                owner_id: Some(Some(target_user_id)),
                ..Default::default()
            },
        )
        .await?;

    let msg = MessageSync::ChannelUpdate {
        channel: Box::new(thread.clone()),
    };
    s.broadcast_channel(req.channel_id, auth.user.id, msg)
        .await?;
    Ok(Json(thread))
}

/// Ratelimit update
#[handler(routes::channel_ratelimit_update)]
async fn channel_ratelimit_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_ratelimit_update::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(Error::MissingPermissions);
    }

    let mut message_expire_at = None;
    let mut thread_expire_at = None;

    if let Some(expire_at_opt) = req.ratelimit.slowmode_message_expire_at {
        if let Some(expire_at) = expire_at_opt {
            s.data()
                .channel_set_message_slowmode_expire_at(req.channel_id, req.user_id, expire_at)
                .await?;
            message_expire_at = Some(expire_at);
        } else {
            s.data()
                .channel_set_message_slowmode_expire_at(
                    req.channel_id,
                    req.user_id,
                    (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1)))
                        .into(),
                )
                .await?;
            message_expire_at = None;
        }
    }

    if let Some(expire_at_opt) = req.ratelimit.slowmode_thread_expire_at {
        if let Some(expire_at) = expire_at_opt {
            s.data()
                .channel_set_thread_slowmode_expire_at(req.channel_id, req.user_id, expire_at)
                .await?;
            thread_expire_at = Some(expire_at);
        } else {
            s.data()
                .channel_set_thread_slowmode_expire_at(
                    req.channel_id,
                    req.user_id,
                    (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1)))
                        .into(),
                )
                .await?;
            thread_expire_at = None;
        }
    }

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::RatelimitUpdate {
            channel_id: req.channel_id,
            user_id: req.user_id,
            slowmode_thread_expire_at: thread_expire_at,
            slowmode_message_expire_at: message_expire_at,
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::RatelimitUpdate {
            channel_id: req.channel_id,
            user_id: req.user_id,
            slowmode_thread_expire_at: thread_expire_at,
            slowmode_message_expire_at: message_expire_at,
        },
    )
    .await?;

    Ok(StatusCode::OK)
}

/// Ratelimit delete
#[handler(routes::channel_ratelimit_delete)]
async fn channel_ratelimit_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_ratelimit_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(Error::MissingPermissions);
    }

    s.data()
        .channel_set_message_slowmode_expire_at(
            req.channel_id,
            req.user_id,
            (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into(),
        )
        .await?;
    s.data()
        .channel_set_thread_slowmode_expire_at(
            req.channel_id,
            req.user_id,
            (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into(),
        )
        .await?;

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::RatelimitDelete {
            channel_id: req.channel_id,
            user_id: req.user_id,
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::RatelimitUpdate {
            channel_id: req.channel_id,
            user_id: req.user_id,
            slowmode_thread_expire_at: None,
            slowmode_message_expire_at: None,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Ratelimit delete all
#[handler(routes::channel_ratelimit_delete_all)]
async fn channel_ratelimit_delete_all(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_ratelimit_delete_all::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(Error::MissingPermissions);
    }

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        data.channel_ratelimit_delete_all(req.channel_id).await?;

        al.commit_success(AuditLogEntryType::RatelimitDeleteAll {
            channel_id: req.channel_id,
        })
        .await?;
    } else {
        data.channel_ratelimit_delete_all(req.channel_id).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(channel_create_room))
        .routes(routes2!(channel_create_dm))
        .routes(routes2!(channel_get))
        .routes(routes2!(channel_list))
        .routes(routes2!(channel_list_removed))
        .routes(routes2!(channel_reorder))
        .routes(routes2!(channel_update))
        .routes(routes2!(channel_ack))
        .routes(routes2!(channel_remove))
        .routes(routes2!(channel_restore))
        .routes(routes2!(channel_typing))
        .routes(routes2!(channel_upgrade))
        .routes(routes2!(channel_transfer_ownership))
        .routes(routes2!(channel_ratelimit_update))
        .routes(routes2!(channel_ratelimit_delete))
        .routes(routes2!(channel_ratelimit_delete_all))
}
