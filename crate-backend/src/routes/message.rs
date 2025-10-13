use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    notifications::{Notification, NotificationReason},
    util::Time,
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ContextQuery, ContextResponse,
    MessageMigrate, MessageModerate, MessagePin, MessageType, NotificationId, PaginationDirection,
    PinsReorder, RepliesQuery, ThreadMemberPut, ThreadMembership, ThreadType,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Error,
    types::{
        DbMessageCreate, Message, MessageCreate, MessageId, MessagePatch, MessageSync,
        MessageVerId, PaginationQuery, PaginationResponse, Permission, ThreadId,
    },
    ServerState,
};

use super::util::{Auth, HeaderIdempotencyKey, HeaderReason};
use crate::error::Result;

/// Message create
///
/// Send a message to a thread
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = [
        "message",
        "badge.perm.MessageCreate",
        "badge.perm-opt.MessageAttachments",
        "badge.perm-opt.MessageEmbeds",
        "badge.perm-opt.MemberBridge",
    ],
    responses(
        (status = CREATED, body = Message, description = "Create message success"),
    )
)]
async fn message_create(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    HeaderIdempotencyKey(nonce): HeaderIdempotencyKey,
    Json(json): Json<MessageCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.ty == ThreadType::Category {
        return Err(Error::BadStatic("cannot send messages in category threads"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
        perms.ensure(Permission::ThreadLock)?;
    }

    let mentions = json.mentions.clone();

    let message = srv
        .messages
        .create(thread_id, auth_user.id, reason, nonce, json)
        .await?;

    let s_clone = s.clone();
    let message_id = message.id;
    let author_id = auth_user.id;
    let room_id = thread.room_id;

    tokio::spawn(async move {
        let mut notified_users = HashSet::new();

        // Direct mentions
        for user_id in mentions.users {
            if user_id == author_id {
                continue;
            }
            if notified_users.insert(user_id) {
                let notification = Notification {
                    id: NotificationId::new(),
                    thread_id,
                    message_id,
                    reason: NotificationReason::Mention,
                    added_at: Time::now_utc(),
                    read_at: None,
                };
                if let Err(e) = s_clone.data().notification_add(user_id, notification).await {
                    tracing::error!(
                        "Failed to add mention notification for user {}: {}",
                        user_id,
                        e
                    );
                }
            }
        }

        // Bulk mentions
        if mentions.everyone_room || mentions.everyone_thread {
            let mut bulk_mention_users = Vec::new();
            if mentions.everyone_room {
                if let Some(room_id) = room_id {
                    let mut after = None;
                    loop {
                        match s_clone
                            .data()
                            .room_member_list(
                                room_id,
                                PaginationQuery {
                                    from: after,
                                    limit: Some(1000),
                                    ..Default::default()
                                },
                            )
                            .await
                        {
                            Ok(page) => {
                                let has_more = page.has_more;
                                let items = page.items;
                                if items.is_empty() {
                                    break;
                                }
                                after = Some(items.last().unwrap().user_id.into());
                                for member in items {
                                    bulk_mention_users.push(member.user_id);
                                }
                                if !has_more {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to get room members for bulk mention: {}",
                                    e
                                );
                                break;
                            }
                        }
                    }
                }
            } else if mentions.everyone_thread {
                let mut after = None;
                loop {
                    match s_clone
                        .data()
                        .thread_member_list(
                            thread_id,
                            PaginationQuery {
                                from: after,
                                limit: Some(1000),
                                ..Default::default()
                            },
                        )
                        .await
                    {
                        Ok(page) => {
                            let has_more = page.has_more;
                            let items = page.items;
                            if items.is_empty() {
                                break;
                            }
                            after = Some(items.last().unwrap().user_id.into());
                            for member in items {
                                bulk_mention_users.push(member.user_id);
                            }
                            if !has_more {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to get thread members for bulk mention: {}", e);
                            break;
                        }
                    }
                }
            }

            for user_id in bulk_mention_users {
                if user_id == author_id {
                    continue;
                }
                if notified_users.insert(user_id) {
                    let notification = Notification {
                        id: NotificationId::new(),
                        thread_id,
                        message_id,
                        reason: NotificationReason::MentionBulk,
                        added_at: Time::now_utc(),
                        read_at: None,
                    };
                    if let Err(e) = s_clone.data().notification_add(user_id, notification).await {
                        tracing::error!(
                            "Failed to add bulk mention notification for user {}: {}",
                            user_id,
                            e
                        );
                    }
                }
            }
        }
    });

    Ok((StatusCode::CREATED, Json(message)))
}

/// Message context
///
/// More efficient than calling List messages twice
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/context/{message_id}",
    params(
        ContextQuery,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = ContextResponse, description = "List thread messages success"),
    )
)]
async fn message_context(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<ContextQuery>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let limit = q.limit.unwrap_or(10);
    if limit > 1024 {
        return Err(Error::BadStatic("limit too big"));
    }
    let before_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_start,
        dir: Some(PaginationDirection::B),
        limit: Some(limit),
    };
    let before = data.message_list(thread_id, auth_user.id, before_q).await?;
    let after_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_end,
        dir: Some(PaginationDirection::F),
        limit: Some(limit),
    };
    let after = data.message_list(thread_id, auth_user.id, after_q).await?;
    let message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await
        .ok();
    let mut res = ContextResponse {
        items: before
            .items
            .into_iter()
            .chain(message)
            .chain(after.items)
            .collect(),
        total: after.total,
        has_after: after.has_more,
        has_before: before.has_more,
    };
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Messages list
///
/// Paginate messages in a thread
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message",
    params(PaginationQuery<MessageId>, ("thread_id", description = "Thread id")),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    )
)]
async fn message_list(
    Path((thread_id,)): Path<(ThreadId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data.message_list(thread_id, auth_user.id, q).await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Message get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "List thread messages success"),
    )
)]
async fn message_get(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await?;
    s.presign_message(&mut message).await?;
    Ok(Json(message))
}

/// Message edit
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "edit message success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn message_edit(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<MessagePatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
        perms.ensure(Permission::ThreadLock)?;
    }

    let (_status, message) = srv
        .messages
        .edit(thread_id, message_id, auth_user.id, reason, json)
        .await?;
    Ok((StatusCode::OK, Json(message)))
}

/// Message delete (TEMP?)
///
/// Note that this endpoint allows deleting your own messages, while message
/// moderate always requires the full permission
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = [
        "message",
        "badge.perm-opt.MessageDelete",
    ],
    tags = ["message"],
    responses(
        (status = NO_CONTENT, description = "delete message success"),
    )
)]
async fn message_delete(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let mut perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    let message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author_id == auth_user.id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    data.message_delete(thread_id, message_id).await?;
    data.media_link_delete_all(message_id.into_inner()).await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageDelete {
                thread_id,
                message_id,
            },
        })
        .await?;
    }

    s.broadcast_thread(
        thread.id,
        auth_user.id,
        MessageSync::MessageDelete {
            room_id: thread.room_id,
            thread_id,
            message_id,
        },
    )
    .await?;
    s.services().threads.invalidate(thread_id).await; // last version id, message count
    Ok(StatusCode::NO_CONTENT)
}

/// Message version list
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/version",
    params(
        PaginationQuery<MessageVerId>,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
async fn message_version_list(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<PaginationQuery<MessageVerId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data
        .message_version_list(thread_id, message_id, auth_user.id, q)
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Message version get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/version/{version_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "success"),
    )
)]
async fn message_version_get(
    Path((thread_id, _message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut message = data
        .message_version_get(thread_id, version_id, auth_user.id)
        .await?;
    s.presign_message(&mut message).await?;
    Ok(Json(message))
}

/// Message moderate
///
/// Bulk remove, restore, or delete messages.
///
/// Deleting a message:
/// - Deleted messages remain visible to moderators (and sender, although there's no ui for this).
/// - Deleted messages cannot be restored by moderators (ask your local server admin if needed).
/// - Deleted messages are garbage collected after 7 days.
///
/// Removing a message:
/// - Removing a message hides it from all non-moderators and the sender.
/// - Removal is reversible via restoration, unlike deletion.
/// - Removed messages are never garbage collected.
/// - This is a "softer" form of deletion, intended for moderators you don't fully trust.
///
/// Permissions:
/// - `MessageDelete` allows deleting messages and viewing deleted messages.
/// - `MessageRemove` allows removing/restoring messages and viewing removed messages.
/// - Users can always delete (but not remove) their own messages.
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = [
        "message",
        "badge.perm-opt.MessageDelete",
        "badge.perm-opt.MessageRemove",
    ],
    responses((status = OK, description = "success")),
)]
async fn message_moderate(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageModerate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;

    if json.delete.is_empty() && json.remove.is_empty() && json.restore.is_empty() {
        return Ok(StatusCode::OK);
    }

    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    if !json.delete.is_empty() {
        perms.ensure(Permission::MessageDelete)?;
        // TODO: fix n+1 query
        for id in &json.delete {
            let message = data.message_get(thread_id, *id, auth_user.id).await?;
            if !message.message_type.is_deletable() {
                return Err(Error::BadStatic("cant delete one of the messages"));
            }
        }

        data.message_delete_bulk(thread_id, &json.delete).await?;
        for id in &json.delete {
            data.media_link_delete_all(id.into_inner()).await?;
        }

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::MessageDeleteBulk {
                    thread_id,
                    message_ids: json.delete.clone(),
                },
            })
            .await?;
        }

        s.broadcast_thread(
            thread.id,
            auth_user.id,
            MessageSync::MessageDeleteBulk {
                thread_id,
                message_ids: json.delete.clone(),
            },
        )
        .await?;
    }

    if !json.remove.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        // TODO: fix n+1 query
        for id in &json.remove {
            let message = data.message_get(thread_id, *id, auth_user.id).await?;
            if !message.message_type.is_deletable() {
                return Err(Error::BadStatic("cant remove one of the messages"));
            }
        }

        data.message_remove_bulk(thread_id, &json.remove).await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::MessageRemove {
                    thread_id,
                    message_ids: json.remove.clone(),
                },
            })
            .await?;
        }

        s.broadcast_thread(
            thread.id,
            auth_user.id,
            MessageSync::MessageRemove {
                thread_id,
                message_ids: json.remove.clone(),
            },
        )
        .await?;
    }

    if !json.restore.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        data.message_restore_bulk(thread_id, &json.restore).await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::MessageRestore {
                    thread_id,
                    message_ids: json.restore.clone(),
                },
            })
            .await?;
        }

        s.broadcast_thread(
            thread.id,
            auth_user.id,
            MessageSync::MessageRestore {
                thread_id,
                message_ids: json.restore.clone(),
            },
        )
        .await?;
    }

    srv.threads.invalidate(thread_id).await; // last version id, message count
    Ok(StatusCode::OK)
}

/// Message move (TODO)
///
/// Move messages from one thread to another. Requires `MessageMove` in both the
/// source and target thread.
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/migrate",
    params(("thread_id", description = "Thread id")),
    tags = [
        "message",
        "badge.perm.MessageMove",
    ],
    responses((status = NO_CONTENT, description = "move success")),
)]
async fn message_migrate(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<MessageMigrate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Message replies
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/reply/{message_id}",
    params(
        RepliesQuery,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    ),
)]
async fn message_replies(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<RepliesQuery>,
    Query(pagination): Query<PaginationQuery<MessageId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data
        .message_replies(
            thread_id,
            Some(message_id),
            auth_user.id,
            q.depth,
            q.breadth,
            pagination,
        )
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Message roots
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/reply",
    params(
        RepliesQuery,
        ("thread_id", description = "Thread id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    ),
)]
async fn message_roots(
    Path((thread_id,)): Path<(ThreadId,)>,
    Query(q): Query<RepliesQuery>,
    Query(pagination): Query<PaginationQuery<MessageId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data
        .message_replies(
            thread_id,
            None,
            auth_user.id,
            q.depth,
            q.breadth,
            pagination,
        )
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Pin create
///
/// - Newly pinned messages are pinned to the top (position 0).
/// - There can be a maximum of 1024 pinned messages.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/pin/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = [
        "message",
        "badge.perm.MessagePin",
    ],
    responses(
        (status = OK, description = "success"),
    ),
)]
async fn message_pin_create(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure(Permission::MessagePin)?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    data.message_pin_create(thread_id, message_id).await?;

    let message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await?;

    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::MessageUpdate { message },
    )
    .await?;

    let notice_message_id = data
        .message_create(DbMessageCreate {
            thread_id,
            attachment_ids: vec![],
            author_id: auth_user.id,
            embeds: vec![],
            message_type: MessageType::MessagePinned(MessagePin {
                pinned_message_id: message_id,
            }),
            edited_at: None,
            created_at: None,
            mentions: Default::default(),
        })
        .await?;
    let mut notice_message = data
        .message_get(thread_id, notice_message_id, auth_user.id)
        .await?;

    let user_id = auth_user.id;
    let tm = data.thread_member_get(thread_id, user_id).await;
    if tm.is_err() || tm.is_ok_and(|tm| tm.membership == ThreadMembership::Leave) {
        data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
            .await?;
        let thread_member = data.thread_member_get(thread_id, user_id).await?;
        let msg = MessageSync::ThreadMemberUpsert {
            member: thread_member,
        };
        s.broadcast_thread(thread_id, user_id, msg).await?;
    }

    s.presign_message(&mut notice_message).await?;
    srv.threads.invalidate(thread_id).await; // message count
    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::MessageCreate {
            message: notice_message,
        },
    )
    .await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessagePin {
                thread_id,
                message_id,
            },
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Pin delete
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/pin/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = [
        "message",
        "badge.perm.MessagePin",
    ],
    responses(
        (status = OK, description = "success"),
    ),
)]
async fn message_pin_delete(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure(Permission::MessagePin)?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    s.data().message_pin_delete(thread_id, message_id).await?;

    let message = s
        .data()
        .message_get(thread_id, message_id, auth_user.id)
        .await?;

    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::MessageUpdate { message },
    )
    .await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageUnpin {
                thread_id,
                message_id,
            },
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Pin reorder
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/pin",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = [
        "message",
        "badge.perm.MessagePin",
    ],
    responses(
        (status = OK, description = "Reorder pinned messages success"),
    ),
)]
async fn message_pin_reorder(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PinsReorder>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure(Permission::MessagePin)?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    s.data()
        .message_pin_reorder(thread_id, json.clone())
        .await?;

    // broadcast update for all affected messages
    for item in json.messages {
        let message = s
            .data()
            .message_get(thread_id, item.id, auth_user.id)
            .await?;
        s.broadcast_thread(
            thread_id,
            auth_user.id,
            MessageSync::MessageUpdate { message },
        )
        .await?;
    }

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessagePinReorder { thread_id },
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Message pin list
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/pin",
    params(
        PaginationQuery<MessageId>,
        ("thread_id", description = "Thread id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List pinned messages success"),
    ),
)]
async fn message_pin_list(
    Path(thread_id): Path<ThreadId>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data.message_pin_list(thread_id, auth_user.id, q).await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(message_create))
        .routes(routes!(message_get))
        .routes(routes!(message_list))
        .routes(routes!(message_context))
        .routes(routes!(message_edit))
        .routes(routes!(message_delete))
        .routes(routes!(message_version_list))
        .routes(routes!(message_version_get))
        .routes(routes!(message_replies))
        .routes(routes!(message_roots))
        .routes(routes!(message_moderate))
        .routes(routes!(message_migrate))
        .routes(routes!(message_pin_create))
        .routes(routes!(message_pin_delete))
        .routes(routes!(message_pin_reorder))
        .routes(routes!(message_pin_list))
}
