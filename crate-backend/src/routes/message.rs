use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Time;
use common::v1::types::{AuditLogEntryType, MessagePin, MessageType, ThreadMemberPut};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::routes::util::Auth;
use crate::routes2;
use crate::types::{DbMessageCreate, MessageSync, Permission};
use crate::{error::Result, Error, ServerState};

/// Message create
#[handler(routes::message_create)]
async fn message_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_create::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    chan.ensure_has_text()?;

    let header_timestamp = req.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let message = srv
        .messages
        .create(
            req.channel_id,
            &auth,
            req.idempotency_key,
            req.message,
            header_timestamp,
        )
        .await?;

    // automatically ack the channel for the user who sent the message
    let data = s.data();
    data.unread_ack(
        auth.user.id,
        req.channel_id,
        message.id,
        message.latest_version.version_id,
        Some(0),
    )
    .await?;
    srv.channels
        .invalidate_user(req.channel_id, auth.user.id)
        .await;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Message context
#[handler(routes::message_context)]
async fn message_context(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_context::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;

    let res = srv
        .messages
        .list_context(req.channel_id, req.message_id, auth.user.id, req.context)
        .await?;

    Ok(Json(res))
}

/// Messages list
#[handler(routes::message_list)]
async fn message_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let res = srv
        .messages
        .list(req.channel_id, Some(auth.user.id), req.pagination)
        .await?;
    Ok(Json(res))
}

/// Message get
#[handler(routes::message_get)]
async fn message_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let message = srv
        .messages
        .get(req.channel_id, req.message_id, auth.user.id)
        .await?;
    Ok(Json(message))
}

/// Message edit
#[handler(routes::message_edit)]
async fn message_edit(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_edit::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure_unlocked()?;
    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    thread.ensure_has_text()?;

    let header_timestamp = req.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let (_status, message) = srv
        .messages
        .edit(
            req.channel_id,
            req.message_id,
            auth.user.id,
            req.patch,
            header_timestamp,
        )
        .await?;
    Ok(Json(message))
}

/// Message delete
#[handler(routes::message_delete)]
async fn message_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let message = data
        .message_get(req.channel_id, req.message_id, auth.user.id)
        .await?;
    if !message.latest_version.message_type.is_deletable() {
        return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
    }
    let is_author = message.author_id == auth.user.id;
    if !perms.has_or(Permission::MessageDelete, is_author) {
        return Err(Error::ApiError(ApiError {
            required_permissions: vec![Permission::MessageDelete],
            ..ApiError::from_code(ErrorCode::MissingPermissions)
        }));
    }

    if message.author_id != auth.user.id {
        if let Some(room_id) = thread.room_id {
            let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
            if room.security.require_mfa {
                let user = srv.users.get(auth.user.id, None).await?;
                let totp = data.auth_totp_get(user.id).await?;
                if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                    return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
                }
            }
        }
    }

    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    thread.ensure_has_text()?;
    perms.ensure_unlocked()?;

    data.message_delete(req.channel_id, req.message_id).await?;
    data.media_link_delete_all(req.message_id.into_inner())
        .await?;

    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MessageDelete {
            channel_id: req.channel_id,
            message_id: req.message_id,
        })
        .await?;
    }

    s.broadcast_channel(
        thread.id,
        auth.user.id,
        MessageSync::MessageDelete {
            channel_id: req.channel_id,
            message_id: req.message_id,
        },
    )
    .await?;
    s.services().channels.invalidate(req.channel_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Message version list
#[handler(routes::message_version_list)]
async fn message_version_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_version_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let res = srv
        .messages
        .list_versions(req.channel_id, req.message_id, auth.user.id, req.pagination)
        .await?;
    Ok(Json(res))
}

/// Message version get
#[handler(routes::message_version_get)]
async fn message_version_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_version_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let message = srv
        .messages
        .get_version(req.channel_id, req.version_id, auth.user.id)
        .await?;
    Ok(Json(message))
}

/// Message version delete
#[handler(routes::message_version_delete)]
async fn message_version_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_version_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    thread.ensure_has_text()?;

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure_unlocked()?;
    perms.ensure(Permission::ChannelView)?;

    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let message = data
        .message_get(req.channel_id, req.message_id, auth.user.id)
        .await?;

    if message.latest_version.version_id == req.version_id {
        return Err(ApiError::from_code(ErrorCode::CannotDeleteLatestMessageVersion).into());
    }

    let version = data
        .message_version_get(req.channel_id, req.version_id, auth.user.id)
        .await?;

    if !version.message_type.is_deletable() {
        return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessageType).into());
    }

    let is_author = message.author_id == auth.user.id;
    if !perms.has_or(Permission::MessageDelete, is_author) {
        return Err(Error::ApiError(ApiError {
            required_permissions: vec![Permission::MessageDelete],
            ..ApiError::from_code(ErrorCode::MissingPermissions)
        }));
    }

    if message.author_id != auth.user.id {
        if let Some(room_id) = thread.room_id {
            let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
            if room.security.require_mfa {
                let user = srv.users.get(auth.user.id, None).await?;
                let totp = data.auth_totp_get(user.id).await?;
                if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                    return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
                }
            }
        }
    }

    data.message_version_delete(req.channel_id, req.version_id)
        .await?;

    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MessageVersionDelete {
            channel_id: req.channel_id,
            message_id: req.message_id,
            version_id: req.version_id,
        })
        .await?;
    }

    s.broadcast_channel(
        thread.id,
        auth.user.id,
        MessageSync::MessageVersionDelete {
            channel_id: req.channel_id,
            message_id: req.message_id,
            version_id: req.version_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Message moderate
#[handler(routes::message_moderate)]
async fn message_moderate(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_moderate::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.moderate.validate()?;

    if req.moderate.delete.is_empty()
        && req.moderate.remove.is_empty()
        && req.moderate.restore.is_empty()
    {
        return Ok(StatusCode::OK);
    }

    let data = s.data();
    let srv = s.services();

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;

    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    thread.ensure_has_text()?;
    perms.ensure_unlocked()?;

    let mut needs_mfa = false;
    if !req.moderate.delete.is_empty() {
        perms.ensure(Permission::MessageDelete)?;
        for id in &req.moderate.delete {
            let message = data.message_get(req.channel_id, *id, auth.user.id).await?;
            if !message.latest_version.message_type.is_deletable() {
                return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
            }
            if message.author_id != auth.user.id {
                needs_mfa = true;
            }
        }
    }

    if !req.moderate.remove.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        for id in &req.moderate.remove {
            let message = data.message_get(req.channel_id, *id, auth.user.id).await?;
            if !message.latest_version.message_type.is_deletable() {
                return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
            }
        }
        needs_mfa = true;
    }

    if !req.moderate.restore.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        needs_mfa = true;
    }

    if needs_mfa {
        if let Some(room_id) = thread.room_id {
            let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
            if room.security.require_mfa {
                let user = srv.users.get(auth.user.id, None).await?;
                let totp = data.auth_totp_get(user.id).await?;
                if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                    return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
                }
            }
        }
    }

    if !req.moderate.delete.is_empty() {
        for id in &req.moderate.delete {
            data.media_link_delete_all(id.into_inner()).await?;
        }

        data.message_delete_bulk(req.channel_id, &req.moderate.delete)
            .await?;

        if let Some(room_id) = thread.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::MessageDeleteBulk {
                channel_id: req.channel_id,
                message_ids: req.moderate.delete.clone(),
            })
            .await?;
        }

        s.broadcast_channel(
            thread.id,
            auth.user.id,
            MessageSync::MessageDeleteBulk {
                channel_id: req.channel_id,
                message_ids: req.moderate.delete.clone(),
            },
        )
        .await?;
    }

    if !req.moderate.remove.is_empty() {
        data.message_remove_bulk(req.channel_id, &req.moderate.remove)
            .await?;

        if let Some(room_id) = thread.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::MessageRemove {
                channel_id: req.channel_id,
                message_ids: req.moderate.remove.clone(),
            })
            .await?;
        }

        s.broadcast_channel(
            thread.id,
            auth.user.id,
            MessageSync::MessageRemove {
                channel_id: req.channel_id,
                message_ids: req.moderate.remove.clone(),
            },
        )
        .await?;
    }

    if !req.moderate.restore.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        data.message_restore_bulk(req.channel_id, &req.moderate.restore)
            .await?;

        if let Some(room_id) = thread.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::MessageRestore {
                channel_id: req.channel_id,
                message_ids: req.moderate.restore.clone(),
            })
            .await?;
        }

        s.broadcast_channel(
            thread.id,
            auth.user.id,
            MessageSync::MessageRestore {
                channel_id: req.channel_id,
                message_ids: req.moderate.restore.clone(),
            },
        )
        .await?;
    }

    srv.channels.invalidate(req.channel_id).await;
    Ok(StatusCode::OK)
}

/// Message migrate
#[handler(routes::message_migrate)]
async fn message_migrate(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::message_migrate::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Message pin
#[handler(routes::message_pin)]
async fn message_pin(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_pin::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::MessagePin)?;

    thread.ensure_has_text()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let created = data
        .message_pin_create(req.channel_id, req.message_id)
        .await?;

    let message = data
        .message_get(req.channel_id, req.message_id, auth.user.id)
        .await?;

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::MessageUpdate { message },
    )
    .await?;

    if !created {
        return Ok(StatusCode::OK);
    }

    let notice_message_id = data
        .message_create(DbMessageCreate {
            id: None,
            channel_id: req.channel_id,
            attachment_ids: vec![],
            author_id: auth.user.id,
            embeds: vec![],
            message_type: MessageType::MessagePinned(MessagePin {
                pinned_message_id: req.message_id,
            })
            .into(),
            created_at: None,
            removed_at: None,
            mentions: Default::default(),
        })
        .await?;
    let mut notice_message = data
        .message_get(req.channel_id, notice_message_id, auth.user.id)
        .await?;

    let user_id = auth.user.id;
    let tm = data.thread_member_get(req.channel_id, user_id).await;
    if tm.is_err() {
        data.thread_member_put(req.channel_id, user_id, ThreadMemberPut::default())
            .await?;
        let thread_member = data.thread_member_get(req.channel_id, user_id).await?;
        let msg = MessageSync::ThreadMemberUpsert {
            room_id: thread.room_id,
            thread_id: req.channel_id,
            added: vec![thread_member],
            removed: vec![],
        };
        s.broadcast_channel(req.channel_id, user_id, msg).await?;
    }

    s.presign_message(&mut notice_message).await?;
    srv.channels.invalidate(req.channel_id).await;
    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::MessageCreate {
            message: notice_message,
        },
    )
    .await?;

    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MessagePin {
            channel_id: req.channel_id,
            message_id: req.message_id,
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Message unpin
#[handler(routes::message_unpin)]
async fn message_unpin(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_unpin::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::MessagePin)?;

    thread.ensure_has_text()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    s.data()
        .message_pin_delete(req.channel_id, req.message_id)
        .await?;

    let message = s
        .data()
        .message_get(req.channel_id, req.message_id, auth.user.id)
        .await?;

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::MessageUpdate { message },
    )
    .await?;

    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MessageUnpin {
            channel_id: req.channel_id,
            message_id: req.message_id,
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Message pins list
#[handler(routes::message_pins_list)]
async fn message_pins_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_pins_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let res = srv
        .messages
        .list_pins(req.channel_id, auth.user.id, req.pagination)
        .await?;
    Ok(Json(res))
}

/// Message pins reorder
#[handler(routes::message_pins_reorder)]
async fn message_pins_reorder(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_pins_reorder::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.reorder.validate()?;
    let srv = s.services();
    let data = s.data();

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::MessagePin)?;

    thread.ensure_has_text()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    s.data()
        .message_pin_reorder(req.channel_id, req.reorder.clone())
        .await?;

    for item in req.reorder.messages {
        let message = s
            .data()
            .message_get(req.channel_id, item.id, auth.user.id)
            .await?;
        s.broadcast_channel(
            req.channel_id,
            auth.user.id,
            MessageSync::MessageUpdate { message },
        )
        .await?;
    }

    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MessagePinReorder {
            channel_id: req.channel_id,
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Message replies list
#[handler(routes::message_replies_list)]
async fn message_replies_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_replies_list::Request,
) -> Result<impl IntoResponse> {
    req.replies.validate()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let res = srv
        .messages
        .list_replies(
            req.channel_id,
            Some(req.message_id),
            auth.user.id,
            req.replies,
            req.pagination,
        )
        .await?;
    Ok(Json(res))
}

/// Message list deleted
#[handler(routes::message_list_deleted)]
async fn message_list_deleted(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_list_deleted::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::MessageDelete)?;
    let res = srv
        .messages
        .list_deleted(req.channel_id, auth.user.id, req.pagination)
        .await?;
    Ok(Json(res))
}

/// Message list removed
#[handler(routes::message_list_removed)]
async fn message_list_removed(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::message_list_removed::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::MessageRemove)?;
    let res = srv
        .messages
        .list_removed(req.channel_id, auth.user.id, req.pagination)
        .await?;
    Ok(Json(res))
}

/// Message list atom/rss (TODO)
#[handler(routes::message_list_atom)]
pub async fn message_list_atom(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::message_list_atom::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Nudge (TODO)
#[handler(routes::message_nudge)]
pub async fn message_nudge(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::message_nudge::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(message_create))
        .routes(routes2!(message_get))
        .routes(routes2!(message_list))
        .routes(routes2!(message_list_deleted))
        .routes(routes2!(message_list_removed))
        .routes(routes2!(message_list_atom))
        .routes(routes2!(message_context))
        .routes(routes2!(message_edit))
        .routes(routes2!(message_delete))
        .routes(routes2!(message_version_list))
        .routes(routes2!(message_version_get))
        .routes(routes2!(message_version_delete))
        .routes(routes2!(message_replies_list))
        .routes(routes2!(message_moderate))
        .routes(routes2!(message_migrate))
        .routes(routes2!(message_pin))
        .routes(routes2!(message_unpin))
        .routes(routes2!(message_pins_reorder))
        .routes(routes2!(message_pins_list))
        .routes(routes2!(message_nudge))
}
