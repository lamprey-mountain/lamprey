use common::{
    v1::types::{
        AuditLogEntryType, MessagePin, MessageSync, MessageType, ThreadMemberCreate,
        ack::{AckBulkItem, AckType},
        oauth::Scope,
        util::Time,
    },
    v2::types::MessageId,
};
use http::StatusCode;
use kerosene_core::types::permission::{CheckPermissions, Permissions2};
use lamprey_backend_services::compat::types::{DbMessageCreate, Permission};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::message_create)]
pub async fn create(
    req: Req<routes::message_create::Endpoint>,
) -> Result<routes::message_create::Response> {
    let identity = req.identity();
    let session = identity.ensure_session()?; // NOTE: make optional?
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;
    user.ensure_unsuspended()?;

    // grab these before into_innering req
    let globals = req.globals();
    let srv = req.services();
    let session_id = session.id;
    let user_id = user.id;

    let body = req.into_inner();
    let channel_id = body.channel_id;

    let timestamp = body.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let message_id = MessageId::new();
    let message = srv
        .messages
        .create3(channel_id, user_id)
        .id(message_id)
        .session(Some(session_id))
        .timestamp(timestamp)
        .nonce(body.idempotency_key)
        .create_default(body.message)
        .await
        .cast_internal()?;

    // return 201 if message was created and 200 if message already existed
    let status = if message.id == message_id {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let message_id = message.id;

    // TODO: move this logic to notifications or ack service
    // automatically ack the channel for the user who sent the message
    let mut txn = globals.begin().await.cast_internal()?;
    txn.unread_ack_bulk(
        user_id,
        &[AckBulkItem {
            ty: AckType::Message {
                channel_id,
                message_id,
                mention_count: 0,
            },
        }],
    )
    .await
    .cast_internal()?;
    txn.commit().await.cast_internal()?;

    srv.channels.invalidate_user(channel_id, user_id).await;

    Ok(routes::message_create::Response { message, status })
}

#[handler(routes::message_get)]
pub async fn get(req: Req<routes::message_get::Endpoint>) -> Result<routes::message_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let user_id = identity.user_id();
    let body = req.inner();

    srv.perms
        .for_channel3(user_id, body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let message = srv
        .messages
        .get(body.channel_id, body.message_id, user_id)
        .await
        .cast_internal()?;

    Ok(routes::message_get::Response { message })
}

#[handler(routes::message_context)]
pub async fn context(
    req: Req<routes::message_context::Endpoint>,
) -> Result<routes::message_context::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let context = srv
        .messages
        .list_context(
            body.channel_id,
            body.message_id,
            identity.user_id(),
            body.context.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::message_context::Response { context })
}

#[handler(routes::message_list)]
pub async fn list(
    req: Req<routes::message_list::Endpoint>,
) -> Result<routes::message_list::Response> {
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

    let messages = srv
        .messages
        .list(body.channel_id, identity.user_id(), body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::message_list::Response { messages })
}

#[handler(routes::message_edit)]
pub async fn edit(
    req: Req<routes::message_edit::Endpoint>,
) -> Result<routes::message_edit::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs_unlocked();

    let thread = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    thread.ensure_has_text()?;

    let header_timestamp = body.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let mut auth = ReqAuth::new(&req);
    let (_status, message) = srv
        .messages
        .edit(
            body.channel_id,
            body.message_id,
            &mut auth,
            body.patch.clone(),
            header_timestamp,
        )
        .await
        .cast_internal()?;

    Ok(routes::message_edit::Response { message })
}

#[handler(routes::message_delete)]
pub async fn delete(
    req: Req<routes::message_delete::Endpoint>,
) -> Result<routes::message_delete::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?.clone();
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let thread = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    let mut perms = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let mut data = globals.begin().await.cast_internal()?;

    let message = data
        .message_get(body.channel_id, body.message_id)
        .await
        .cast_internal()?;
    if !message.latest_version.message_type.is_deletable() {
        return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
    }

    let is_author = message.author_id == auth_user.id;
    if !is_author {
        perms.needs(Permission::MessageDelete);
        if let Some(room_id) = thread.room_id {
            let room = srv
                .rooms
                .get(room_id, Some(auth_user.id))
                .await
                .cast_internal()?;
            if room.security.require_mfa {
                let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
                if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                    return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
                }
            }
        }
    }

    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    thread.ensure_has_text()?;
    perms.needs_unlocked();
    perms.check()?;

    data.message_delete(body.channel_id, body.message_id)
        .await
        .cast_internal()?;
    data.media_link_delete_all(body.message_id.into_inner())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    if let Some(room_id) = thread.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::MessageDelete {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            thread.id,
            MessageSync::MessageDelete {
                channel_id: body.channel_id,
                message_id: body.message_id,
                room_id: thread.room_id,
            },
        )
        .await
        .cast_internal()?;

    srv.channels.invalidate(body.channel_id).await;

    Ok(routes::message_delete::Response {})
}

#[handler(routes::message_version_list)]
pub async fn version_list(
    req: Req<routes::message_version_list::Endpoint>,
) -> Result<routes::message_version_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let user_id = identity.user_id();

    srv.perms
        .for_channel3(user_id, body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let versions = srv
        .messages
        .list_versions(
            body.channel_id,
            body.message_id,
            user_id,
            body.pagination.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::message_version_list::Response { versions })
}

#[handler(routes::message_version_get)]
pub async fn version_get(
    req: Req<routes::message_version_get::Endpoint>,
) -> Result<routes::message_version_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let user_id = identity.user_id();

    srv.perms
        .for_channel3(user_id, body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let message = srv
        .messages
        .get_version(body.channel_id, body.version_id, user_id)
        .await
        .cast_internal()?;

    Ok(routes::message_version_get::Response { message })
}

#[handler(routes::message_version_delete)]
pub async fn version_delete(
    req: Req<routes::message_version_delete::Endpoint>,
) -> Result<routes::message_version_delete::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?.clone();
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let thread = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    thread.ensure_has_text()?;

    let mut perms = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs_unlocked();

    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    let message = data
        .message_get(body.channel_id, body.message_id)
        .await
        .cast_internal()?;

    if message.latest_version.version_id == body.version_id {
        return Err(ApiError::from_code(ErrorCode::CannotDeleteLatestMessageVersion).into());
    }

    let version = data
        .message_version_get(body.channel_id, body.version_id)
        .await
        .cast_internal()?;

    if !version.message_type.is_deletable() {
        return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessageType).into());
    }

    let is_author = message.author_id == auth_user.id;
    if !perms.has(Permission::MessageDelete) && !is_author {
        return Err(ApiError {
            required_permissions: vec![Permission::MessageDelete],
            ..ApiError::from_code(ErrorCode::MissingPermissions)
        }
        .into());
    }

    if message.author_id != auth_user.id {
        if let Some(room_id) = thread.room_id {
            let room = srv
                .rooms
                .get(room_id, Some(auth_user.id))
                .await
                .cast_internal()?;
            if room.security.require_mfa {
                let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
                if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                    return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
                }
            }
        }
    }

    data.message_version_delete(body.channel_id, body.version_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    if let Some(room_id) = thread.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::MessageVersionDelete {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                    version_id: body.version_id,
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            thread.id,
            MessageSync::MessageVersionDelete {
                channel_id: body.channel_id,
                message_id: body.message_id,
                version_id: body.version_id,
                room_id: thread.room_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::message_version_delete::Response {})
}

#[handler(routes::message_moderate)]
pub async fn moderate(
    req: Req<routes::message_moderate::Endpoint>,
) -> Result<routes::message_moderate::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?.clone();
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.moderate
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    if body.moderate.delete.is_empty()
        && body.moderate.remove.is_empty()
        && body.moderate.restore.is_empty()
    {
        return Ok(routes::message_moderate::Response {});
    }

    let globals = req.globals();
    let srv = req.services();

    let thread = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    let mut perms = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs_unlocked();

    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    thread.ensure_has_text()?;

    let mut data = globals.begin().await.cast_internal()?;

    let mut needs_mfa = false;
    if !body.moderate.delete.is_empty() {
        perms.needs(Permission::MessageDelete);
        for id in &body.moderate.delete {
            let message = data
                .message_get(body.channel_id, *id)
                .await
                .cast_internal()?;
            if !message.latest_version.message_type.is_deletable() {
                return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
            }
            if message.author_id != auth_user.id {
                needs_mfa = true;
            }
        }
    }

    if !body.moderate.remove.is_empty() {
        perms.needs(Permission::MessageRemove);
        for id in &body.moderate.remove {
            let message = data
                .message_get(body.channel_id, *id)
                .await
                .cast_internal()?;
            if !message.latest_version.message_type.is_deletable() {
                return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
            }
        }
        needs_mfa = true;
    }

    if !body.moderate.restore.is_empty() {
        perms.needs(Permission::MessageRemove);
        needs_mfa = true;
    }

    perms.check()?;

    if needs_mfa {
        if let Some(room_id) = thread.room_id {
            let room = srv
                .rooms
                .get(room_id, Some(auth_user.id))
                .await
                .cast_internal()?;
            if room.security.require_mfa {
                let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
                if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                    return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
                }
            }
        }
    }

    if !body.moderate.delete.is_empty() {
        for id in &body.moderate.delete {
            data.media_link_delete_all(id.into_inner())
                .await
                .cast_internal()?;
        }
        data.message_delete_bulk(body.channel_id, &body.moderate.delete)
            .await
            .cast_internal()?;
    }

    if !body.moderate.remove.is_empty() {
        data.message_remove_bulk(body.channel_id, &body.moderate.remove)
            .await
            .cast_internal()?;
    }

    if !body.moderate.restore.is_empty() {
        data.message_restore_bulk(body.channel_id, &body.moderate.restore)
            .await
            .cast_internal()?;
    }

    data.commit().await.cast_internal()?;

    if !body.moderate.delete.is_empty() {
        if let Some(room_id) = thread.room_id {
            req.audit_log()
                .push(
                    room_id,
                    AuditLogEntryType::MessageDeleteBulk {
                        channel_id: body.channel_id,
                        message_ids: body.moderate.delete.clone(),
                    },
                )
                .success();
        }

        globals
            .messaging()
            .broadcast_channel(
                thread.id,
                MessageSync::MessageDeleteBulk {
                    channel_id: body.channel_id,
                    message_ids: body.moderate.delete.clone(),
                    room_id: thread.room_id,
                },
            )
            .await
            .cast_internal()?;
    }

    if !body.moderate.remove.is_empty() {
        if let Some(room_id) = thread.room_id {
            req.audit_log()
                .push(
                    room_id,
                    AuditLogEntryType::MessageRemove {
                        channel_id: body.channel_id,
                        message_ids: body.moderate.remove.clone(),
                    },
                )
                .success();
        }

        globals
            .messaging()
            .broadcast_channel(
                thread.id,
                MessageSync::MessageRemove {
                    channel_id: body.channel_id,
                    message_ids: body.moderate.remove.clone(),
                    room_id: thread.room_id,
                },
            )
            .await
            .cast_internal()?;
    }

    if !body.moderate.restore.is_empty() {
        if let Some(room_id) = thread.room_id {
            req.audit_log()
                .push(
                    room_id,
                    AuditLogEntryType::MessageRestore {
                        channel_id: body.channel_id,
                        message_ids: body.moderate.restore.clone(),
                    },
                )
                .success();
        }

        globals
            .messaging()
            .broadcast_channel(
                thread.id,
                MessageSync::MessageRestore {
                    channel_id: body.channel_id,
                    message_ids: body.moderate.restore.clone(),
                    room_id: thread.room_id,
                },
            )
            .await
            .cast_internal()?;
    }

    srv.channels.invalidate(body.channel_id).await;

    Ok(routes::message_moderate::Response {})
}

#[handler(routes::message_move)]
pub async fn move_message(
    _req: Req<routes::message_move::Endpoint>,
) -> Result<routes::message_move::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::message_pin)]
pub async fn pin(req: Req<routes::message_pin::Endpoint>) -> Result<routes::message_pin::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?.clone();
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let thread = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if let Some(room_id) = thread.room_id {
        let room = srv
            .rooms
            .get(room_id, Some(auth_user.id))
            .await
            .cast_internal()?;
        if room.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    srv.perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::MessagePin)
        .check()?;

    thread.ensure_has_text()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    let created = data
        .message_pin_create(body.channel_id, body.message_id)
        .await
        .cast_internal()?;

    let message = data
        .message_get(body.channel_id, body.message_id)
        .await
        .cast_internal()?;

    if !created {
        data.commit().await.cast_internal()?;
        globals
            .messaging()
            .broadcast_channel(body.channel_id, MessageSync::MessageUpdate { message })
            .await
            .cast_internal()?;
        return Ok(routes::message_pin::Response {});
    }

    let notice_message_id = data
        .message_create(DbMessageCreate {
            id: None,
            channel_id: body.channel_id,
            attachments: vec![],
            author_id: auth_user.id,
            embeds: vec![],
            components: vec![],
            message_type: MessageType::MessagePinned(MessagePin {
                pinned_message_id: body.message_id,
            })
            .into(),
            created_at: None,
            removed_at: None,
            flume: None,
            mentions: Default::default(),
            interaction: None,
            ephemeral: false,
        })
        .await
        .cast_internal()?;
    let notice_message = data
        .message_get(body.channel_id, notice_message_id)
        .await
        .cast_internal()?;

    let user_id = auth_user.id;
    let tm = data.thread_member_get(body.channel_id, user_id).await;
    if tm.is_err() {
        data.thread_member_put(body.channel_id, user_id, ThreadMemberCreate::default())
            .await
            .cast_internal()?;
        let thread_member = data
            .thread_member_get(body.channel_id, user_id)
            .await
            .cast_internal()?;
        data.commit().await.cast_internal()?;

        globals
            .messaging()
            .broadcast_channel(
                body.channel_id,
                MessageSync::ThreadMemberUpsert {
                    room_id: thread.room_id,
                    thread_id: body.channel_id,
                    added: vec![thread_member],
                    removed: vec![],
                },
            )
            .await
            .cast_internal()?;
    } else {
        data.commit().await.cast_internal()?;
    }

    srv.channels.invalidate(body.channel_id).await;

    globals
        .messaging()
        .broadcast_channel(body.channel_id, MessageSync::MessageUpdate { message })
        .await
        .cast_internal()?;
    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::MessageCreate {
                message: notice_message,
            },
        )
        .await
        .cast_internal()?;

    if let Some(room_id) = thread.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::MessagePin {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                },
            )
            .success();
    }

    Ok(routes::message_pin::Response {})
}

#[handler(routes::message_unpin)]
pub async fn unpin(
    req: Req<routes::message_unpin::Endpoint>,
) -> Result<routes::message_unpin::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?.clone();
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let thread = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if let Some(room_id) = thread.room_id {
        let room = srv
            .rooms
            .get(room_id, Some(auth_user.id))
            .await
            .cast_internal()?;
        if room.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    srv.perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::MessagePin)
        .check()?;

    thread.ensure_has_text()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.message_pin_delete(body.channel_id, body.message_id)
        .await
        .cast_internal()?;
    let message = data
        .message_get(body.channel_id, body.message_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    globals
        .messaging()
        .broadcast_channel(body.channel_id, MessageSync::MessageUpdate { message })
        .await
        .cast_internal()?;

    if let Some(room_id) = thread.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::MessageUnpin {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                },
            )
            .success();
    }

    Ok(routes::message_unpin::Response {})
}

#[handler(routes::message_pins_list)]
pub async fn pins_list(
    req: Req<routes::message_pins_list::Endpoint>,
) -> Result<routes::message_pins_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let pins = srv
        .messages
        .list_pins(body.channel_id, auth_user.id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::message_pins_list::Response { pins })
}

#[handler(routes::message_pins_reorder)]
pub async fn pins_reorder(
    req: Req<routes::message_pins_reorder::Endpoint>,
) -> Result<routes::message_pins_reorder::Response> {
    let identity = req.identity();
    let auth_user = identity.ensure_user()?.clone();
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.reorder
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let globals = req.globals();
    let srv = req.services();

    let thread = srv
        .channels
        .get(body.channel_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if let Some(room_id) = thread.room_id {
        let room = srv
            .rooms
            .get(room_id, Some(auth_user.id))
            .await
            .cast_internal()?;
        if room.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    srv.perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::MessagePin)
        .check()?;

    thread.ensure_has_text()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.message_pin_reorder(body.channel_id, body.reorder.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    for item in &body.reorder.messages {
        let message = globals
            .begin_read()
            .await
            .cast_internal()?
            .message_get(body.channel_id, item.id)
            .await
            .cast_internal()?;
        globals
            .messaging()
            .broadcast_channel(body.channel_id, MessageSync::MessageUpdate { message })
            .await
            .cast_internal()?;
    }

    if let Some(room_id) = thread.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::MessagePinReorder {
                    channel_id: body.channel_id,
                },
            )
            .success();
    }

    Ok(routes::message_pins_reorder::Response {})
}

#[handler(routes::message_reply_roots)]
pub async fn reply_roots(
    req: Req<routes::message_reply_roots::Endpoint>,
) -> Result<routes::message_reply_roots::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();
    body.replies
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let _perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let replies = srv
        .messages
        .list_replies(
            body.channel_id,
            None,
            auth_user.id,
            body.replies.clone(),
            body.pagination.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::message_reply_roots::Response { replies })
}

#[handler(routes::message_reply_list)]
pub async fn reply_list(
    req: Req<routes::message_reply_list::Endpoint>,
) -> Result<routes::message_reply_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();
    body.replies
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let _perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let replies = srv
        .messages
        .list_replies(
            body.channel_id,
            Some(body.message_id),
            auth_user.id,
            body.replies.clone(),
            body.pagination.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::message_reply_list::Response { replies })
}

#[handler(routes::message_list_deleted)]
pub async fn list_deleted(
    req: Req<routes::message_list_deleted::Endpoint>,
) -> Result<routes::message_list_deleted::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();

    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::MessageDelete).check()?;

    let messages = srv
        .messages
        .list_deleted(body.channel_id, Some(auth_user.id), body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::message_list_deleted::Response { messages })
}

#[handler(routes::message_list_atom)]
pub async fn list_atom(
    _req: Req<routes::message_list_atom::Endpoint>,
) -> Result<routes::message_list_atom::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::message_list_removed)]
pub async fn list_removed(
    req: Req<routes::message_list_removed::Endpoint>,
) -> Result<routes::message_list_removed::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();

    let mut perms: Permissions2<CheckPermissions> = srv
        .perms
        .for_channel3(Some(auth_user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::MessageRemove).check()?;

    let messages = srv
        .messages
        .list_removed(body.channel_id, Some(auth_user.id), body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::message_list_removed::Response { messages })
}

#[handler(routes::message_nudge)]
pub async fn nudge(
    req: Req<routes::message_nudge::Endpoint>,
) -> Result<routes::message_nudge::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?.clone();
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let user_id = user.id;
    let channel_id = body.channel_id;

    use lamprey_backend_services::services::messages::create2::CreateType;

    let chan = srv.channels.get(channel_id, None).await.cast_internal()?;
    if !chan.is_dm() {
        return Err(Error::internal_message("you can only send nudges in dms"));
    }

    let session_id = identity.session().map(|s| s.id);

    let message = srv
        .messages
        .create3(channel_id, user.id)
        .session(session_id)
        .nonce(body.idempotency_key.clone())
        .create(CreateType::Custom(MessageType::Nudge))
        .await
        .cast_internal()?;

    let mut txn = globals.begin().await.cast_internal()?;
    txn.unread_ack_bulk(
        user_id,
        &[AckBulkItem {
            ty: AckType::Message {
                channel_id,
                message_id: message.id,
                mention_count: 0,
            },
        }],
    )
    .await
    .cast_internal()?;
    txn.commit().await.cast_internal()?;

    srv.channels.invalidate_user(channel_id, user_id).await;

    Ok(routes::message_nudge::Response {
        message,
        status: StatusCode::CREATED,
    })
}

export_routes!(
    create,
    get,
    context,
    list,
    edit,
    delete,
    version_list,
    version_get,
    version_delete,
    moderate,
    move_message,
    pin,
    unpin,
    pins_list,
    pins_reorder,
    reply_roots,
    reply_list,
    list_deleted,
    list_atom,
    list_removed,
    nudge
);
