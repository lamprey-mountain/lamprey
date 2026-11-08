use std::collections::HashMap;

use common::v1::types::{
    AuditLogEntryType, ChannelReorderItem, ChannelType, MessageSync, Permission, RelationshipType,
    RoomCreate, RoomMemberOrigin, RoomType, SERVER_ROOM_ID, ThreadMemberCreate,
    ack::{AckBulkItem, AckState, AckType},
    application::Scope,
    pagination::{PaginationQuery, PaginationResponse},
    util::Changes,
};
use http::StatusCode;
use lamprey_backend_services::compat::{
    consts::MAX_GDM_MEMBERS,
    types::{ChannelPatch, DbChannelCreate, DbChannelType, DbRoomCreate, MediaLinkType},
};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::channel_create_room)]
pub async fn create_room(
    req: Req<routes::channel_create_room::Endpoint>,
) -> Result<routes::channel_create_room::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    room.room_type.ensure_channels_manageable()?;

    body.channel
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let mut auth = ReqAuth::new(&req);
    let channel = srv
        .channels
        .create(
            &mut auth,
            Some(body.room_id),
            body.channel.clone(),
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_create_room::Response { channel })
}

#[handler(routes::channel_create_dm)]
pub async fn create_dm(
    req: Req<routes::channel_create_dm::Endpoint>,
) -> Result<routes::channel_create_dm::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let user = identity.ensure_user()?.clone();
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    body.channel
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    srv.perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::DmCreate)
        .check()?;

    let mut create = body.channel.clone();

    match create.ty {
        ChannelType::Dm => {
            let Some(recipients) = &create.recipients else {
                return Err(ApiError::from_code(ErrorCode::DmThreadMissingRecipients).into());
            };
            if recipients.len() != 1 {
                return Err(ApiError::from_code(ErrorCode::DmThreadSinglePersonOnly).into());
            }
            let target_user_id = recipients[0];

            let (thread, is_new) = srv
                .users
                .init_dm(user.id, target_user_id, false)
                .await
                .cast_internal()?;

            globals
                .messaging()
                .broadcast_global(MessageSync::ChannelCreate {
                    channel: Box::new(thread.clone()),
                })
                .await
                .cast_internal()?;

            let status = if is_new {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };

            return Ok(routes::channel_create_dm::Response {
                channel: thread,
                status,
            });
        }
        ChannelType::Gdm => {
            if create.name.is_none() {
                return Err(ApiError::from_code(ErrorCode::FieldMissingName).into());
            }

            let mut recipients = create.recipients.clone().unwrap_or_default();
            recipients.push(user.id);

            if recipients.len() as u32 > MAX_GDM_MEMBERS {
                return Err(ApiError::from_code(ErrorCode::GdmTooManyMembers).into());
            }

            let mut data = globals.begin().await.cast_internal()?;
            for recipient_id in recipients.iter().filter(|id| **id != user.id) {
                let relationship = data
                    .user_relationship_get(user.id, *recipient_id)
                    .await
                    .cast_internal()?;

                let are_friends =
                    relationship.is_some_and(|r| r.relation == Some(RelationshipType::Friend));

                if !are_friends {
                    return Err(ApiError::from_code(ErrorCode::GdmRequiresFriend).into());
                }
            }

            create.recipients = Some(recipients);

            if create.bitrate.is_some_and(|b| b > 393216) {
                return Err(ApiError::from_code(ErrorCode::BitrateTooHigh).into());
            }
            if !create.ty.has_voice() && create.bitrate.is_some() {
                return Err(
                    ApiError::from_code(ErrorCode::CannotSetBitrateForNonVoiceThread).into(),
                );
            }
            if !create.ty.has_voice() && create.user_limit.is_some() {
                return Err(
                    ApiError::from_code(ErrorCode::CannotSetUserLimitForNonVoiceThread).into(),
                );
            }

            if let Some(icon) = create.icon {
                create.ty.ensure_has_icon()?;
                let media = data.media_select(icon).await.cast_internal()?;
                if !media.metadata.is_image() {
                    return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
                }
            }

            let channel_id = data
                .channel_create(DbChannelCreate {
                    room_id: None,
                    creator_id: user.id,
                    name: create.name.clone(),
                    description: create.description.clone(),
                    auto_name: create.auto_name,
                    auto_description: create.auto_description,
                    icon: create.icon.map(|i| *i),
                    ty: DbChannelType::Gdm,
                    nsfw: create.nsfw,
                    bitrate: create.bitrate.map(|b| b as i32),
                    user_limit: create.user_limit.map(|u| u as i32),
                    parent_id: None,
                    owner_id: Some(*user.id),
                    invitable: create.invitable,
                    auto_archive_duration: create.auto_archive_duration.map(|d| d as i64),
                    default_auto_archive_duration: create
                        .default_auto_archive_duration
                        .flatten()
                        .map(|d| d as i64),
                    slowmode_thread: create.slowmode_thread.map(|d| d as i64),
                    slowmode_message: create.slowmode_message.map(|d| d as i64),
                    default_slowmode_message: create.default_slowmode_message.map(|d| d as i64),
                    tags: create.tags.clone(),
                    url: create.url.clone(),
                    locked: false,
                })
                .await
                .cast_internal()?;

            if let Some(icon) = create.icon {
                data.media_link_create_exclusive(icon, *channel_id, MediaLinkType::ChannelIcon)
                    .await
                    .cast_internal()?;
            }

            let thread = srv
                .channels
                .get(channel_id, Some(user.id))
                .await
                .cast_internal()?;
            let mut members = vec![];

            if let Some(recipients) = &create.recipients {
                for id in recipients {
                    data.thread_member_put(channel_id, *id, ThreadMemberCreate {})
                        .await
                        .cast_internal()?;
                    let thread_member = data
                        .thread_member_get(channel_id, *id)
                        .await
                        .cast_internal()?;
                    members.push(thread_member);
                }
            }
            data.commit().await.cast_internal()?;

            globals
                .messaging()
                .broadcast_global(MessageSync::ChannelCreate {
                    channel: Box::new(thread.clone()),
                })
                .await
                .cast_internal()?;
            if !members.is_empty() {
                globals
                    .messaging()
                    .broadcast_global(MessageSync::ThreadMemberUpsert {
                        room_id: thread.room_id,
                        thread_id: thread.id,
                        added: members,
                        removed: vec![],
                    })
                    .await
                    .cast_internal()?;
            }

            Ok(routes::channel_create_dm::Response {
                channel: thread,
                status: StatusCode::CREATED,
            })
        }
        _ => Err(ApiError::from_code(ErrorCode::DmGdmOnlyOutsideRoom).into()),
    }
}

#[handler(routes::channel_get)]
pub async fn get(req: Req<routes::channel_get::Endpoint>) -> Result<routes::channel_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let channel = srv
        .channels
        .get(body.channel_id, identity.user_id())
        .await
        .cast_internal()?;

    Ok(routes::channel_get::Response { channel })
}

#[handler(routes::channel_list)]
pub async fn list(
    req: Req<routes::channel_list::Endpoint>,
) -> Result<routes::channel_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let room = srv
        .rooms
        .load(body.room_id)
        .ready(true)
        .await
        .cast_internal()?;
    let visible_channel_ids: Vec<_> = room
        .channel_visibilities(identity.user_id())
        .into_iter()
        .map(|visibility| visibility.id)
        .collect();
    let channels = srv
        .channels
        .get_many(&visible_channel_ids, identity.user_id())
        .await
        .cast_internal()?;
    let total = channels.len() as u64;

    Ok(routes::channel_list::Response {
        channels: PaginationResponse {
            items: channels,
            total,
            has_more: false,
            cursor: None,
        },
    })
}

#[handler(routes::channel_list_removed)]
pub async fn list_removed(
    req: Req<routes::channel_list_removed::Endpoint>,
) -> Result<routes::channel_list_removed::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::ChannelManage)
        .check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let mut res = data
        .channel_list_removed(body.room_id, body.pagination.clone(), body.query.parent_id)
        .await
        .cast_internal()?;

    let mut items = Vec::new();
    for item in res.items {
        if srv
            .perms
            .for_channel3(Some(auth_user.id), item.id)
            .await
            .cast_internal()?
            .visible
        {
            items.push(item);
        }
    }
    res.items = items;

    let ids: Vec<_> = res.items.iter().map(|t| t.id).collect();
    let channels = srv
        .channels
        .get_many(&ids, Some(auth_user.id))
        .await
        .cast_internal()?;
    let mut channels_map: HashMap<_, _> = channels.into_iter().map(|c| (c.id, c)).collect();
    res.items = ids
        .into_iter()
        .filter_map(|id| channels_map.remove(&id))
        .collect();

    Ok(routes::channel_list_removed::Response {
        channels: PaginationResponse {
            items: res.items,
            total: res.total,
            has_more: res.has_more,
            cursor: res.cursor,
        },
    })
}

#[handler(routes::channel_reorder)]
pub async fn reorder(
    req: Req<routes::channel_reorder::Endpoint>,
) -> Result<routes::channel_reorder::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::ChannelManage)
        .check()?;

    let mut channels_old = HashMap::new();

    for channel in &body.reorder.channels {
        let channel_data = srv.channels.get(channel.id, None).await.cast_internal()?;
        channels_old.insert(channel_data.id, channel_data.clone());

        srv.perms
            .for_channel3(Some(user.id), channel.id)
            .await
            .cast_internal()?
            .ensure_view()?
            .needs(Permission::ChannelManage)
            .check()?;

        if let Some(Some(parent_id)) = channel.parent_id {
            srv.perms
                .for_channel3(Some(user.id), parent_id)
                .await
                .cast_internal()?
                .ensure_view()?
                .needs(Permission::ChannelManage)
                .check()?;

            let parent_data = srv.channels.get(parent_id, None).await.cast_internal()?;
            if !channel_data.ty.can_be_in(Some(parent_data.ty)) {
                return Err(ApiError::from_code(ErrorCode::InvalidParentChannelType).into());
            }
        }
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.channel_reorder(body.reorder.clone())
        .await
        .cast_internal()?;
    data.room_template_mark_dirty(body.room_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    let mut changed_channels = Vec::new();
    for chan in &body.reorder.channels {
        srv.channels.invalidate(chan.id).await;
        let chan_old = channels_old.get(&chan.id);
        let chan = srv.channels.get(chan.id, None).await.cast_internal()?;
        if let Some(chan_old) = chan_old
            && chan.parent_id == chan_old.parent_id
            && chan.position == chan_old.position
        {
            continue;
        }
        changed_channels.push(ChannelReorderItem {
            id: chan.id,
            parent_id: Some(chan.parent_id),
            position: Some(chan.position),
        });
    }

    if !changed_channels.is_empty() {
        globals
            .messaging()
            .broadcast_room(
                body.room_id,
                MessageSync::ChannelReorder {
                    room_id: body.room_id,
                    channels: changed_channels.clone(),
                },
            )
            .await
            .cast_internal()?;

        req.audit_log()
            .push(
                body.room_id,
                AuditLogEntryType::ChannelReorder {
                    channels: changed_channels,
                },
            )
            .success();
    }

    Ok(routes::channel_reorder::Response {})
}

#[handler(routes::channel_update)]
pub async fn update(
    req: Req<routes::channel_update::Endpoint>,
) -> Result<routes::channel_update::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;
    if body.patch.owner_id.is_some() {
        return Err(ApiError::from_code(ErrorCode::OwnerIdCannotBeChanged).into());
    }

    let globals = req.globals();
    let srv = req.services();
    let chan_pre = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    if let Some(room_id) = chan_pre.room_id {
        srv.perms
            .for_room3(Some(user.id), room_id)
            .await
            .cast_internal()?
            .ensure_view()?;
    }

    let mut auth = ReqAuth::new(&req);
    let chan = srv
        .channels
        .update(&mut auth, body.channel_id, body.patch.clone())
        .await
        .cast_internal()?;

    if let Some(icon) = body.patch.icon {
        let mut data = globals.begin().await.cast_internal()?;
        data.media_link_delete(*body.channel_id, MediaLinkType::ChannelIcon)
            .await
            .cast_internal()?;
        if let Some(icon) = icon {
            data.media_link_create_exclusive(icon, *body.channel_id, MediaLinkType::ChannelIcon)
                .await
                .cast_internal()?;
        }
        data.commit().await.cast_internal()?;
    }

    Ok(routes::channel_update::Response { channel: chan })
}

#[handler(routes::channel_ack)]
pub async fn ack(req: Req<routes::channel_ack::Endpoint>) -> Result<routes::channel_ack::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?.clone();
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let message_id = if let Some(id) = body.ack.message_id {
        id
    } else {
        let chan = srv
            .channels
            .get(body.channel_id, None)
            .await
            .cast_internal()?;
        match chan.last_message_id {
            Some(id) => id,
            None => {
                return Err(Error::internal_message(
                    "channel doesnt have a last_message_id",
                ));
            }
        }
    };

    let ack_ty = AckType::Message {
        channel_id: body.channel_id,
        message_id,
        mention_count: body.ack.mention_count,
    };

    let mut data = globals.begin().await.cast_internal()?;
    data.unread_ack_bulk(auth_user.id, &[AckBulkItem { ty: ack_ty.clone() }])
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.channels
        .invalidate_user(body.channel_id, auth_user.id)
        .await;

    let state = AckState {
        ty: ack_ty,
        unread: false,
    };

    globals
        .messaging()
        .broadcast_user(
            auth_user.id,
            MessageSync::PassiveAck {
                user_id: auth_user.id,
                ack_states: vec![state.clone()],
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_ack::Response { ack: state })
}

#[handler(routes::channel_remove)]
pub async fn remove(
    req: Req<routes::channel_remove::Endpoint>,
) -> Result<routes::channel_remove::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?.clone();
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let channel_id = req.inner().channel_id;

    let chan_before = srv
        .channels
        .get(channel_id, Some(user.id))
        .await
        .cast_internal()?;
    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    if chan_before.is_thread() {
        perms.needs(Permission::ThreadManage);
    } else {
        perms.needs(Permission::ChannelManage);
    }
    perms.check()?;

    if let Some(room_id) = chan_before.room_id {
        let room = srv
            .rooms
            .get(room_id, Some(user.id))
            .await
            .cast_internal()?;
        if room.security.require_sudo {
            identity.ensure_sudo()?;
        }
        if room.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let chan_before = srv
            .channels
            .get(channel_id, Some(user.id))
            .await
            .cast_internal()?;
        if chan_before.is_removed() {
            return Ok(routes::channel_remove::Response {});
        }

        let mut data = globals.begin().await.cast_internal()?;
        data.channel_delete(channel_id).await.cast_internal()?;
        data.room_template_mark_dirty(room_id)
            .await
            .cast_internal()?;
        data.commit().await.cast_internal()?;

        srv.channels.invalidate(channel_id).await;
        srv.voice
            .call_disconnect_all(channel_id)
            .await
            .cast_internal()?;

        let chan = srv
            .channels
            .get(channel_id, Some(user.id))
            .await
            .cast_internal()?;

        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::ChannelUpdate {
                    channel_id,
                    channel_type: chan.ty,
                    changes: Changes::new()
                        .change("deleted_at", &chan_before.deleted_at, &chan.deleted_at)
                        .build(),
                },
            )
            .success();

        globals
            .messaging()
            .broadcast_room(
                room_id,
                MessageSync::ChannelUpdate {
                    channel: Box::new(chan),
                },
            )
            .await
            .cast_internal()?;
    } else {
        let chan_before = srv
            .channels
            .get(channel_id, Some(user.id))
            .await
            .cast_internal()?;
        if chan_before.is_removed() {
            return Ok(routes::channel_remove::Response {});
        }

        let mut data = globals.begin().await.cast_internal()?;
        data.channel_delete(channel_id).await.cast_internal()?;
        data.commit().await.cast_internal()?;

        srv.channels.invalidate(channel_id).await;
        srv.voice
            .call_disconnect_all(channel_id)
            .await
            .cast_internal()?;
    }

    Ok(routes::channel_remove::Response {})
}

#[handler(routes::channel_restore)]
pub async fn restore(
    req: Req<routes::channel_restore::Endpoint>,
) -> Result<routes::channel_restore::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?.clone();
    let globals = req.globals();
    let srv = req.services();
    let channel_id = req.inner().channel_id;

    let channel = srv
        .channels
        .get(channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if let Some(room_id) = channel.room_id {
        let room = srv
            .rooms
            .get(room_id, Some(auth_user.id))
            .await
            .cast_internal()?;
        if room.security.require_sudo {
            identity.ensure_sudo()?;
        }

        let mut perms = srv
            .perms
            .for_channel3(Some(auth_user.id), channel_id)
            .await
            .cast_internal()?
            .ensure_view()?;
        if channel.is_thread() {
            perms.needs(Permission::ThreadManage);
        } else {
            perms.needs(Permission::ChannelManage);
        }
        perms.check()?;

        if room.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }

        let chan_before = srv
            .channels
            .get(channel_id, Some(auth_user.id))
            .await
            .cast_internal()?;
        if !chan_before.is_removed() {
            return Ok(routes::channel_restore::Response {});
        }

        let mut data = globals.begin().await.cast_internal()?;
        data.channel_undelete(channel_id).await.cast_internal()?;
        data.room_template_mark_dirty(room_id)
            .await
            .cast_internal()?;
        data.commit().await.cast_internal()?;

        srv.channels.invalidate(channel_id).await;
        let chan = srv
            .channels
            .get(channel_id, Some(auth_user.id))
            .await
            .cast_internal()?;

        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::ChannelUpdate {
                    channel_id,
                    channel_type: chan.ty,
                    changes: Changes::new()
                        .change("deleted_at", &chan_before.deleted_at, &chan.deleted_at)
                        .build(),
                },
            )
            .success();

        globals
            .messaging()
            .broadcast_room(
                room_id,
                MessageSync::ChannelUpdate {
                    channel: Box::new(chan),
                },
            )
            .await
            .cast_internal()?;
    } else {
        let mut perms = srv
            .perms
            .for_channel3(Some(auth_user.id), channel_id)
            .await
            .cast_internal()?
            .ensure_view()?;
        if channel.is_thread() {
            perms.needs(Permission::ThreadManage);
        } else {
            perms.needs(Permission::ChannelManage);
        }
        perms.check()?;

        let chan_before = srv
            .channels
            .get(channel_id, Some(auth_user.id))
            .await
            .cast_internal()?;
        if !chan_before.is_removed() {
            return Ok(routes::channel_restore::Response {});
        }

        let mut data = globals.begin().await.cast_internal()?;
        data.channel_undelete(channel_id).await.cast_internal()?;
        data.commit().await.cast_internal()?;

        srv.channels.invalidate(channel_id).await;
    }

    Ok(routes::channel_restore::Response {})
}

#[handler(routes::channel_typing)]
pub async fn typing(
    req: Req<routes::channel_typing::Endpoint>,
) -> Result<routes::channel_typing::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let channel_id = req.inner().channel_id;

    let mut perms = srv
        .perms
        .for_channel3(Some(auth_user.id), channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms
        .needs(Permission::MessageCreate)
        .needs_unlocked()
        .check()?;

    let thread = srv
        .channels
        .get(channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    srv.channels
        .typing_set(channel_id, auth_user.id, until)
        .await;

    let until_time: common::v1::types::util::Time = until
        .try_into()
        .expect("typing indicator time is always valid");

    globals
        .messaging()
        .broadcast_channel(
            channel_id,
            MessageSync::ChannelTyping {
                channel_id,
                user_id: auth_user.id,
                until: until_time,
                room_id: thread.room_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_typing::Response {})
}

#[handler(routes::channel_upgrade)]
pub async fn upgrade(
    req: Req<routes::channel_upgrade::Endpoint>,
) -> Result<routes::channel_upgrade::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?.clone();
    user.ensure_unsuspended()?;
    let user_id = user.id;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let channel_id = req.inner().channel_id;

    let chan = srv
        .channels
        .get(channel_id, Some(user.id))
        .await
        .cast_internal()?;

    if chan.ty != ChannelType::Gdm {
        return Err(ApiError::from_code(ErrorCode::OnlyGdmCanUpgrade).into());
    }
    if chan.owner_id != Some(user.id) {
        return Err(ApiError::from_code(ErrorCode::NotThreadOwner).into());
    }
    if chan.room_id.is_some() {
        return Err(ApiError::from_code(ErrorCode::ThreadAlreadyInRoom).into());
    }

    if let Some(_icon) = chan.icon {
        let mut txn = globals.begin().await.cast_internal()?;
        txn.media_link_delete(*channel_id, MediaLinkType::ChannelIcon)
            .await
            .cast_internal()?;
        txn.commit().await.cast_internal()?;
    }

    let mut auth = ReqAuth::new(&req);
    let handle = srv
        .rooms
        .create(
            RoomCreate {
                name: chan
                    .name
                    .clone()
                    .or_else(|| chan.auto_name.clone())
                    .ok_or_else(|| ApiError::from_code(ErrorCode::FieldMissingName))?,
                description: chan.description.clone(),
                icon: chan.icon,
                banner: None,
                public: Some(false),
            },
            &mut auth,
            DbRoomCreate {
                id: None,
                ty: RoomType::Default,
                welcome_channel_id: Some(channel_id),
                remote: None,
            },
            None,
        )
        .await
        .cast_internal()?;
    let loaded = handle.ready(true).await.cast_internal()?;
    let room = (*loaded.room).clone();

    let mut txn = globals.begin().await.cast_internal()?;
    let mut members = vec![];
    let mut after: Option<common::v1::types::UserId> = None;
    loop {
        let page = txn
            .thread_member_list(
                channel_id,
                PaginationQuery {
                    limit: Some(100),
                    from: after,
                    ..Default::default()
                },
            )
            .await
            .cast_internal()?;

        if page.items.is_empty() {
            break;
        }

        after = Some(page.items.last().unwrap().user_id.into());

        let page_len = page.items.len();
        members.extend(page.items);

        if page_len < 100 {
            break;
        }
    }

    txn.channel_upgrade_gdm(channel_id, room.id)
        .await
        .cast_internal()?;

    for member in &members {
        txn.room_member_put(
            room.id,
            member.user_id,
            Some(RoomMemberOrigin::GdmUpgrade),
            Default::default(),
        )
        .await
        .cast_internal()?;
    }
    txn.commit().await.cast_internal()?;

    srv.rooms.unload_cache(room.id).await;
    let handle = srv.rooms.load(room.id);
    let loaded = handle.ready(true).await.cast_internal()?;
    let room = (*loaded.room).clone();

    srv.channels.invalidate(channel_id).await;
    let upgraded_thread = srv
        .channels
        .get(channel_id, Some(user_id))
        .await
        .cast_internal()?;

    globals
        .messaging()
        .broadcast_global(MessageSync::ChannelUpdate {
            channel: Box::new(upgraded_thread),
        })
        .await
        .cast_internal()?;

    for member in members {
        let cached_member = loaded.members.get(&member.user_id).unwrap();
        globals
            .messaging()
            .broadcast_room(
                room.id,
                MessageSync::RoomMemberCreate {
                    member: cached_member.member.clone(),
                    user: (*cached_member.user).clone(),
                },
            )
            .await
            .cast_internal()?;
    }

    req.audit_log()
        .push(
            room.id,
            AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: ChannelType::Text,
                changes: Changes::new()
                    .change("type", &chan.ty, &ChannelType::Text)
                    .change("room_id", &chan.room_id, &Some(room.id))
                    .build(),
            },
        )
        .success();

    Ok(routes::channel_upgrade::Response { room })
}

#[handler(routes::channel_transfer_ownership)]
pub async fn transfer_ownership(
    req: Req<routes::channel_transfer_ownership::Endpoint>,
) -> Result<routes::channel_transfer_ownership::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;
    identity.ensure_sudo()?;

    let user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.owner_id;

    let mut data = globals.begin_read().await.cast_internal()?;
    data.thread_member_get(body.channel_id, target_user_id)
        .await
        .cast_internal()?;

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?;

    let thread_start = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    if thread_start.owner_id != Some(user.id) {
        return Err(ApiError::from_code(ErrorCode::NotThreadOwner).into());
    }

    let mut auth = ReqAuth::new(&req);
    let thread = srv
        .channels
        .update(
            &mut auth,
            body.channel_id,
            ChannelPatch {
                owner_id: Some(Some(target_user_id)),
                ..Default::default()
            },
        )
        .await
        .cast_internal()?;

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::ChannelUpdate {
                channel: Box::new(thread.clone()),
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_transfer_ownership::Response { channel: thread })
}

#[handler(routes::channel_ratelimit_update)]
pub async fn ratelimit_update(
    req: Req<routes::channel_ratelimit_update::Endpoint>,
) -> Result<routes::channel_ratelimit_update::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let perms = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let mut message_expire_at = None;
    let mut thread_expire_at = None;

    let mut data = globals.begin().await.cast_internal()?;

    if let Some(expire_at_opt) = body.ratelimit.slowmode_message_expire_at {
        if let Some(expire_at) = expire_at_opt {
            data.channel_set_message_slowmode_expire_at(body.channel_id, body.user_id, expire_at)
                .await
                .cast_internal()?;
            message_expire_at = Some(expire_at);
        } else {
            data.channel_set_message_slowmode_expire_at(
                body.channel_id,
                body.user_id,
                (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into(),
            )
            .await
            .cast_internal()?;
            message_expire_at = None;
        }
    }

    if let Some(expire_at_opt) = body.ratelimit.slowmode_thread_expire_at {
        if let Some(expire_at) = expire_at_opt {
            data.channel_set_thread_slowmode_expire_at(body.channel_id, body.user_id, expire_at)
                .await
                .cast_internal()?;
            thread_expire_at = Some(expire_at);
        } else {
            data.channel_set_thread_slowmode_expire_at(
                body.channel_id,
                body.user_id,
                (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into(),
            )
            .await
            .cast_internal()?;
            thread_expire_at = None;
        }
    }

    data.commit().await.cast_internal()?;

    let channel = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if let Some(room_id) = channel.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::RatelimitUpdate {
                    channel_id: body.channel_id,
                    user_id: body.user_id,
                    slowmode_thread_expire_at: thread_expire_at,
                    slowmode_message_expire_at: message_expire_at,
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::RatelimitUpdate {
                channel_id: body.channel_id,
                user_id: body.user_id,
                slowmode_thread_expire_at: thread_expire_at,
                slowmode_message_expire_at: message_expire_at,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_ratelimit_update::Response {})
}

#[handler(routes::channel_ratelimit_delete)]
pub async fn ratelimit_delete(
    req: Req<routes::channel_ratelimit_delete::Endpoint>,
) -> Result<routes::channel_ratelimit_delete::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let perms = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let expired =
        (time::OffsetDateTime::now_utc().saturating_sub(time::Duration::seconds(1))).into();

    let mut data = globals.begin().await.cast_internal()?;
    data.channel_set_message_slowmode_expire_at(body.channel_id, body.user_id, expired)
        .await
        .cast_internal()?;
    data.channel_set_thread_slowmode_expire_at(body.channel_id, body.user_id, expired)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    let channel = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if let Some(room_id) = channel.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::RatelimitDelete {
                    channel_id: body.channel_id,
                    user_id: body.user_id,
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::RatelimitUpdate {
                channel_id: body.channel_id,
                user_id: body.user_id,
                slowmode_thread_expire_at: None,
                slowmode_message_expire_at: None,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_ratelimit_delete::Response {})
}

#[handler(routes::channel_ratelimit_delete_all)]
pub async fn ratelimit_delete_all(
    req: Req<routes::channel_ratelimit_delete_all::Endpoint>,
) -> Result<routes::channel_ratelimit_delete_all::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let channel_id = req.inner().channel_id;

    let perms = srv
        .perms
        .for_channel3(Some(auth_user.id), channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    if !perms.has(Permission::ChannelManage)
        && !perms.has(Permission::ThreadManage)
        && !perms.has(Permission::MemberTimeout)
    {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let channel = srv
        .channels
        .get(channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.channel_ratelimit_delete_all(channel_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    if let Some(room_id) = channel.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::RatelimitDeleteAll { channel_id },
            )
            .success();
    }

    Ok(routes::channel_ratelimit_delete_all::Response {})
}

#[handler(routes::channel_mirror)]
pub async fn mirror(
    req: Req<routes::channel_mirror::Endpoint>,
) -> Result<routes::channel_mirror::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let sync = req
        .globals()
        .begin_read()
        .await
        .cast_internal()?
        .channel_sync(
            body.channel_id,
            body.since.since,
            body.pagination.clone(),
            identity.user_id(),
        )
        .await
        .cast_internal()?;

    Ok(routes::channel_mirror::Response { sync })
}

export_routes!(
    create_room,
    create_dm,
    get,
    list,
    list_removed,
    reorder,
    update,
    ack,
    remove,
    restore,
    typing,
    upgrade,
    transfer_ownership,
    ratelimit_update,
    ratelimit_delete,
    ratelimit_delete_all,
    mirror
);
