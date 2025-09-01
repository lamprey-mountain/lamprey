use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageId, ThreadType,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    types::{
        DbThreadCreate, DbThreadType, MessageSync, MessageVerId, Permission, RoomId, Thread,
        ThreadCreate, ThreadId, ThreadPatch,
    },
    Error, ServerState,
};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};

use super::util::{Auth, HeaderReason};
use crate::error::Result;

/// Room thread create
///
/// Create a thread in a room
#[utoipa::path(
    post,
    path = "/room/{room_id}/thread",
    params(("room_id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = CREATED, body = Thread, description = "Create thread success"),
    )
)]
async fn thread_create_room(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    match json.ty {
        ThreadType::Chat => {
            perms.ensure(Permission::ThreadCreateChat)?;
        }
        ThreadType::Forum => {
            perms.ensure(Permission::ThreadCreateForumTree)?;
        }
        ThreadType::Voice => {
            perms.ensure(Permission::ThreadCreateVoice)?;
        }
        ThreadType::Dm | ThreadType::Gdm => {
            return Err(Error::BadStatic(
                "can't create a direct message thread in a room",
            ))
        }
    };
    let thread_id = data
        .thread_create(DbThreadCreate {
            room_id: Some(room_id.into_inner()),
            creator_id: user_id,
            name: json.name.clone(),
            description: json.description.clone(),
            ty: match json.ty {
                ThreadType::Chat => DbThreadType::Chat,
                ThreadType::Forum => DbThreadType::Forum,
                ThreadType::Voice => DbThreadType::Voice,
                ThreadType::Dm | ThreadType::Gdm => {
                    // this should be unreachable due to the check above
                    warn!("unreachable: dm/gdm thread creation in room");
                    return Err(Error::BadStatic(
                        "can't create a direct message thread in a room",
                    ));
                }
            },
            nsfw: json.nsfw,
        })
        .await?;
    // let starter_message_id = data
    //     .message_create(DbMessageCreate {
    //         thread_id,
    //         attachment_ids: vec![],
    //         author_id: user_id,
    //         embeds: vec![],
    //         message_type: MessageType::ThreadRename(MessageThreadRename {
    //             patch: ThreadPatch {
    //                 name: Some(json.name),
    //                 description: Some(json.description),
    //                 tags: None,
    //                 nsfw: Some(json.nsfw),
    //             },
    //         }),
    //         edited_at: None,
    //         created_at: None,
    //     })
    //     .await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    data.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::ThreadCreate {
            thread_id,
            changes: Changes::new()
                .add("name", &thread.name)
                .add("description", &thread.description)
                .add("nsfw", &thread.nsfw)
                .build(),
        },
    })
    .await?;

    // let starter_message = data
    //     .message_get(thread_id, starter_message_id, user_id)
    //     .await?;
    s.broadcast_room(
        room_id,
        user_id,
        MessageSync::ThreadCreate {
            thread: thread.clone(),
        },
    )
    .await?;

    // s.broadcast_thread(
    //     thread.id,
    //     user_id,
    //     MessageSync::MessageCreate {
    //         message: starter_message,
    //     },
    // )
    // .await?;
    Ok((StatusCode::CREATED, Json(thread)))
}

/// Dm thread create (TODO)
///
/// Create a thread outside of a room, for dms
#[utoipa::path(
    post,
    path = "/thread",
    tags = ["thread"],
    responses(
        (status = CREATED, body = Thread, description = "Create thread success"),
    )
)]
async fn thread_create(
    Path((_room_id,)): Path<(RoomId,)>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(_json): Json<ThreadCreate>,
) -> Result<()> {
    todo!()
}

/// Thread get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "Get thread success"),
    )
)]
async fn thread_get(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    Ok((StatusCode::OK, Json(thread)))
}

/// Room thread list
// maybe in the future i'll replace this with a more flexible "thread query/search" api
#[utoipa::path(
    get,
    path = "/room/{room_id}/thread",
    params(PaginationQuery<ThreadId>, ("room_id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<Thread>, description = "List room threads success"),
    )
)]
async fn thread_list(
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<PaginationQuery<ThreadId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    let mut res = data.thread_list(room_id, q).await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        // FIXME: dubious performance
        threads.push(srv.threads.get(t.id, Some(user_id)).await?);
    }
    res.items = threads;
    Ok(Json(res))
}

/// Room thread list archived
#[utoipa::path(
    get,
    path = "/room/{room_id}/thread/archived",
    params(PaginationQuery<ThreadId>, ("room_id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<Thread>, description = "List archived room threads success"),
    )
)]
async fn thread_list_archived(
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<PaginationQuery<ThreadId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    let mut res = data.thread_list_archived(room_id, q).await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.threads.get(t.id, Some(user_id)).await?);
    }
    res.items = threads;
    Ok(Json(res))
}

/// Room thread list removed
///
/// List removed threads in a room. Requires the `ThreadRemove` permission.
#[utoipa::path(
    get,
    path = "/room/{room_id}/thread/removed",
    params(PaginationQuery<ThreadId>, ("room_id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = OK, body = PaginationResponse<Thread>, description = "List removed room threads success"),
    )
)]
async fn thread_list_removed(
    Path((_room_id,)): Path<(RoomId,)>,
    Query(_q): Query<PaginationQuery<ThreadId>>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    // let data = s.data();
    // let perms = s.services().perms.for_room(user_id, room_id).await?;
    // perms.ensure_view()?;
    // let mut res = data.thread_list_removed(room_id, q).await?;
    Err(Error::Unimplemented)
}

/// Thread edit
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "edit message success"),
        (status = NOT_MODIFIED, body = Thread, description = "no change"),
    )
)]
async fn thread_update(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let thread = s
        .services()
        .threads
        .update(user_id, thread_id, json, reason)
        .await?;
    Ok(Json(thread))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckReq {
    /// The last read message id. Will be resolved from version_id if empty.
    message_id: Option<MessageId>,

    /// The last read id in this thread. Currently unused, may be deprecated later?
    version_id: MessageVerId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckRes {
    /// The last read message id
    message_id: MessageId,

    /// The last read id in this thread. Currently unused, may be deprecated later?.
    version_id: MessageVerId,
}

/// Thread ack
///
/// Mark a thread as read (or unread).
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/ack",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn thread_ack(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AckReq>,
) -> Result<Json<AckRes>> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let version_id = json.version_id;
    let message_id = if let Some(message_id) = json.message_id {
        message_id
    } else {
        data.message_version_get(thread_id, version_id, user_id)
            .await?
            .id
    };
    data.unread_put(user_id, thread_id, message_id, version_id)
        .await?;
    s.services()
        .threads
        .invalidate_user(thread_id, user_id)
        .await;
    Ok(Json(AckRes {
        message_id,
        version_id,
    }))
}

/// Thread archive
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/archive",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_archive(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let thread_before = srv.threads.get(thread_id, Some(user_id)).await?;
    let perms = srv.perms.for_thread(user_id, thread_id).await?;
    if user_id != thread_before.creator_id {
        perms.ensure(Permission::ThreadArchive)?;
    }
    if thread_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread_before.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    if thread_before.archived_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }

    data.thread_archive(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    srv.users.disconnect_everyone_from_thread(thread_id)?;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ThreadUpdate {
                thread_id,
                changes: Changes::new()
                    .change(
                        "archived_at",
                        &thread_before.archived_at,
                        &thread.archived_at,
                    )
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(room_id, user_id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread unarchive
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/archive",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_unarchive(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let thread_before = srv.threads.get(thread_id, Some(user_id)).await?;
    let perms = srv.perms.for_thread(user_id, thread_id).await?;
    if user_id != thread_before.creator_id {
        perms.ensure(Permission::ThreadArchive)?;
    }
    if thread_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread_before.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    if thread_before.archived_at.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_unarchive(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ThreadUpdate {
                thread_id,
                changes: Changes::new()
                    .change(
                        "archived_at",
                        &thread_before.archived_at,
                        &thread.archived_at,
                    )
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(room_id, user_id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread remove
// NOTE: this isn't DELETE. in the future, i probably want to be able to add/remove threads in rooms instead of globally, eg.
// PUT /room/{room_id}/thread/{thread_id}
// DELETE /room/{room_id}/thread/{thread_id}
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/remove",
    params(("thread_id", description = "Thread id")),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_remove(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(user_id, thread_id).await?;
    perms.ensure(Permission::ThreadDelete)?;
    let thread_before = srv.threads.get(thread_id, Some(user_id)).await?;
    if thread_before.deleted_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_delete(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    srv.users.disconnect_everyone_from_thread(thread_id)?;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ThreadUpdate {
                thread_id,
                changes: Changes::new()
                    .change("deleted_at", &thread_before.deleted_at, &thread.deleted_at)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(room_id, user_id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread restore
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/remove",
    params(("thread_id", description = "Thread id")),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_restore(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_thread(user_id, thread_id).await?;
    perms.ensure(Permission::ThreadDelete)?;
    let thread_before = srv.threads.get(thread_id, Some(user_id)).await?;
    if thread_before.deleted_at.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_undelete(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ThreadUpdate {
                thread_id,
                changes: Changes::new()
                    .change("deleted_at", &thread_before.deleted_at, &thread.deleted_at)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(room_id, user_id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread trigger typing indicator
///
/// Send a typing notification to a thread
#[utoipa::path(
    method(post),
    path = "/thread/{thread_id}/typing",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn thread_typing(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageCreate)?;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
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
    srv.threads.typing_set(thread_id, user_id, until).await;
    s.broadcast_thread(
        thread_id,
        user_id,
        MessageSync::ThreadTyping {
            thread_id,
            user_id,
            until: until.into(),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Thread lock
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/lock",
    params(("thread_id", description = "Thread id")),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_lock(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let thread_before = srv.threads.get(thread_id, None).await?;
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ThreadLock)?;
    if thread_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread_before.locked {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_lock(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    srv.users.disconnect_everyone_from_thread(thread_id)?;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ThreadUpdate {
                thread_id,
                changes: Changes::new()
                    .change("locked", &thread_before.locked, &thread.locked)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(room_id, user_id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread unlock
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/lock",
    params(("thread_id", description = "Thread id")),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_unlock(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let thread_before = srv.threads.get(thread_id, None).await?;
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ThreadLock)?;
    if thread_before.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if !thread_before.locked {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_unlock(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    srv.users.disconnect_everyone_from_thread(thread_id)?;
    let thread = srv.threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::ThreadUpdate {
                thread_id,
                changes: Changes::new()
                    .change("locked", &thread_before.locked, &thread.locked)
                    .build(),
            },
        })
        .await?;
        s.broadcast_room(room_id, user_id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_create_room))
        .routes(routes!(thread_create))
        .routes(routes!(thread_get))
        .routes(routes!(thread_list))
        .routes(routes!(thread_list_archived))
        .routes(routes!(thread_list_removed))
        .routes(routes!(thread_update))
        .routes(routes!(thread_ack))
        .routes(routes!(thread_archive))
        .routes(routes!(thread_unarchive))
        .routes(routes!(thread_remove))
        .routes(routes!(thread_restore))
        .routes(routes!(thread_typing))
        .routes(routes!(thread_lock))
        .routes(routes!(thread_unlock))
}
