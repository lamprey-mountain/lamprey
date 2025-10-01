use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    util::Changes, voice::SfuCommand, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageId,
    ThreadMemberPut, ThreadReorder, ThreadType,
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
    tags = [
        "thread",
        "badge.perm-opt.ThreadCreateChat",
        "badge.perm-opt.ThreadCreateForumTree",
        "badge.perm-opt.ThreadCreateVoice",
    ],
    responses(
        (status = CREATED, body = Thread, description = "Create thread success"),
    )
)]
async fn thread_create_room(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let data = s.data();
    let perms = if let Some(parent_id) = json.parent_id {
        srv.perms.for_thread(auth_user.id, parent_id).await?
    } else {
        srv.perms.for_room(auth_user.id, room_id).await?
    };
    perms.ensure_view()?;
    match json.ty {
        ThreadType::Chat => {
            perms.ensure(Permission::ThreadCreateChat)?;
        }
        ThreadType::Forum => {
            perms.ensure(Permission::ThreadCreateForum)?;
        }
        ThreadType::Voice => {
            perms.ensure(Permission::ThreadCreateVoice)?;
        }
        ThreadType::Category => {
            perms.ensure(Permission::ThreadManage)?;
        }
        ThreadType::Dm | ThreadType::Gdm => {
            return Err(Error::BadStatic(
                "can't create a direct message thread in a room",
            ))
        }
    };
    if json.bitrate.is_some_and(|b| b > 393216) {
        return Err(Error::BadStatic("bitrate is too high"));
    }
    if json.ty != ThreadType::Voice && json.bitrate.is_some() {
        return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
    }
    if json.ty != ThreadType::Voice && json.user_limit.is_some() {
        return Err(Error::BadStatic(
            "cannot set user_limit for non voice thread",
        ));
    }
    let thread_id = data
        .thread_create(DbThreadCreate {
            room_id: Some(room_id.into_inner()),
            creator_id: auth_user.id,
            name: json.name.clone(),
            description: json.description.clone(),
            ty: match json.ty {
                ThreadType::Chat => DbThreadType::Chat,
                ThreadType::Forum => DbThreadType::Forum,
                ThreadType::Voice => DbThreadType::Voice,
                ThreadType::Category => DbThreadType::Category,
                ThreadType::Dm | ThreadType::Gdm => {
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
        })
        .await?;

    data.thread_member_put(thread_id, auth_user.id, ThreadMemberPut {})
        .await?;
    let thread_member = data.thread_member_get(thread_id, auth_user.id).await?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::ThreadCreate {
            thread_id,
            changes: Changes::new()
                .add("name", &thread.name)
                .add("description", &thread.description)
                .add("nsfw", &thread.nsfw)
                .add("user_limit", &thread.user_limit)
                .add("bitrate", &thread.bitrate)
                .build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user.id,
        MessageSync::ThreadCreate {
            thread: thread.clone(),
        },
    )
    .await?;
    s.broadcast_thread(
        thread.id,
        auth_user.id,
        MessageSync::ThreadMemberUpsert {
            member: thread_member,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(thread)))
}

/// Dm thread create
///
/// Create a dm or group dm thread (outside of a room)
#[utoipa::path(
    post,
    path = "/thread",
    tags = ["thread"],
    responses(
        (status = CREATED, body = Thread, description = "Create thread success"),
    )
)]
async fn dm_thread_create(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(mut json): Json<ThreadCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let data = s.data();
    match json.ty {
        ThreadType::Dm => {
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
            s.broadcast(MessageSync::ThreadCreate {
                thread: thread.clone(),
            })?;
            if is_new {
                return Ok((StatusCode::CREATED, Json(thread)));
            } else {
                return Ok((StatusCode::OK, Json(thread)));
            }
        }
        ThreadType::Gdm => {
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
    if json.ty != ThreadType::Voice && json.bitrate.is_some() {
        return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
    }
    if json.ty != ThreadType::Voice && json.user_limit.is_some() {
        return Err(Error::BadStatic(
            "cannot set user_limit for non voice thread",
        ));
    }

    let thread_id = data
        .thread_create(DbThreadCreate {
            room_id: None,
            creator_id: auth_user.id,
            name: json.name.clone(),
            description: json.description.clone(),
            ty: DbThreadType::Gdm,
            nsfw: json.nsfw,
            bitrate: json.bitrate.map(|b| b as i32),
            user_limit: json.bitrate.map(|u| u as i32),
            parent_id: None,
        })
        .await?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    let mut members = vec![];

    if let Some(recipients) = &json.recipients {
        for id in recipients {
            data.thread_member_put(thread_id, *id, ThreadMemberPut {})
                .await?;
            let thread_member = data.thread_member_get(thread_id, *id).await?;
            members.push(thread_member);
        }
    }

    s.broadcast(MessageSync::ThreadCreate {
        thread: thread.clone(),
    })?;
    for member in members {
        s.broadcast(MessageSync::ThreadMemberUpsert { member })?;
    }

    Ok((StatusCode::CREATED, Json(thread)))
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let thread = s
        .services()
        .threads
        .get(thread_id, Some(auth_user.id))
        .await?;
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    let mut res = data.thread_list(room_id, q).await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        // FIXME: dubious performance
        threads.push(srv.threads.get(t.id, Some(auth_user.id)).await?);
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    let mut res = data.thread_list_archived(room_id, q).await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.threads.get(t.id, Some(auth_user.id)).await?);
    }
    res.items = threads;
    Ok(Json(res))
}

/// Room thread list removed
///
/// List removed threads in a room. Requires the `ThreadDelete` permission.
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
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<PaginationQuery<ThreadId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ThreadRemove)?;
    let mut res = data.thread_list_removed(room_id, q).await?;
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.threads.get(t.id, Some(auth_user.id)).await?);
    }
    res.items = threads;
    Ok(Json(res))
}

/// Room thread reorder
///
/// Reorder the threads in a room. Requires the `ThreadManage` permission.
#[utoipa::path(
    patch,
    path = "/room/{room_id}/thread",
    params(("room_id", description = "Room id")),
    tags = ["thread", "badge.perm.ThreadReorder"],
    responses(
        (status = OK, body = (), description = "Reorder threads success"),
    )
)]
async fn thread_reorder(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadReorder>,
) -> Result<()> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;

    let mut threads_old = HashMap::new();

    for thread in &json.threads {
        let thread_data = srv.threads.get(thread.id, None).await?;
        threads_old.insert(thread_data.id, thread_data);

        let perms_thread = srv.perms.for_thread(auth_user.id, thread.id).await?;
        perms_thread.ensure_view()?;
        perms_thread.ensure(Permission::ThreadManage)?;

        if let Some(Some(parent_id)) = thread.parent_id {
            let perms_parent = srv.perms.for_thread(auth_user.id, parent_id).await?;
            perms_parent.ensure_view()?;
            perms_parent.ensure(Permission::ThreadManage)?;

            let parent_data = srv.threads.get(parent_id, None).await?;
            if parent_data.ty != ThreadType::Category {
                return Err(Error::BadStatic(
                    "threads can only be children of category threads",
                ));
            }
        }
    }

    data.thread_reorder(json.clone()).await?;

    for thread in &json.threads {
        srv.threads.invalidate(thread.id).await;
        let thread_old = threads_old.get(&thread.id);
        let thread = srv.threads.get(thread.id, None).await?;
        if let Some(thread_old) = thread_old {
            if thread.parent_id == thread_old.parent_id && thread.position == thread_old.position {
                continue;
            }
        }
        s.broadcast_room(room_id, auth_user.id, MessageSync::ThreadUpdate { thread })
            .await?;
    }

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::ThreadReorder {
            threads: json.threads,
        },
    })
    .await?;

    Ok(())
}

/// Thread edit
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread", "badge.perm-opt.ThreadEdit"],
    responses(
        (status = OK, body = Thread, description = "edit message success"),
        (status = NOT_MODIFIED, body = Thread, description = "no change"),
    )
)]
async fn thread_update(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ThreadPatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let thread = s
        .services()
        .threads
        .update(auth_user.id, thread_id, json, reason)
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AckReq>,
) -> Result<Json<AckRes>> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let version_id = json.version_id;
    let message_id = if let Some(message_id) = json.message_id {
        message_id
    } else {
        data.message_version_get(thread_id, version_id, auth_user.id)
            .await?
            .id
    };
    data.unread_put(auth_user.id, thread_id, message_id, version_id)
        .await?;
    s.services()
        .threads
        .invalidate_user(thread_id, auth_user.id)
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
    tags = ["thread", "badge.perm-opt.ThreadArchive"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_archive(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let thread_before = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    if auth_user.id != thread_before.creator_id {
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
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
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
        s.broadcast_room(
            room_id,
            auth_user.id,
            MessageSync::ThreadUpdate {
                thread: thread.clone(),
            },
        )
        .await?;
        s.sushi_sfu
            .send(SfuCommand::Thread {
                thread: thread.into(),
            })
            .unwrap();
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
    tags = ["thread", "badge.perm-opt.ThreadArchive"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_unarchive(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let thread_before = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    if auth_user.id != thread_before.creator_id {
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
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
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
        s.broadcast_room(room_id, auth_user.id, MessageSync::ThreadUpdate { thread })
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
    tags = ["thread", "badge.perm.ThreadDelete"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_remove(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure(Permission::ThreadRemove)?;
    let thread_before = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread_before.deleted_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_delete(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    srv.users.disconnect_everyone_from_thread(thread_id)?;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
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
        s.broadcast_room(room_id, auth_user.id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread restore
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/remove",
    params(("thread_id", description = "Thread id")),
    tags = ["thread", "badge.perm.ThreadDelete"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_restore(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure(Permission::ThreadRemove)?;
    let thread_before = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread_before.deleted_at.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }
    data.thread_undelete(thread_id).await?;
    srv.threads.invalidate(thread_id).await;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
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
        s.broadcast_room(room_id, auth_user.id, MessageSync::ThreadUpdate { thread })
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
    tags = ["thread", "badge.perm.MessageCreate"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn thread_typing(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageCreate)?;
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
    let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    srv.threads.typing_set(thread_id, auth_user.id, until).await;
    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::ThreadTyping {
            thread_id,
            user_id: auth_user.id,
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
    tags = ["thread", "badge.perm.ThreadLock"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_lock(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let thread_before = srv.threads.get(thread_id, None).await?;
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
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
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
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
        s.broadcast_room(room_id, auth_user.id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Thread unlock
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/lock",
    params(("thread_id", description = "Thread id")),
    tags = ["thread", "badge.perm.ThreadLock"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn thread_unlock(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let thread_before = srv.threads.get(thread_id, None).await?;
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
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
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
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
        s.broadcast_room(room_id, auth_user.id, MessageSync::ThreadUpdate { thread })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_create_room))
        .routes(routes!(dm_thread_create))
        .routes(routes!(thread_get))
        .routes(routes!(thread_list))
        .routes(routes!(thread_list_archived))
        .routes(routes!(thread_list_removed))
        .routes(routes!(thread_reorder))
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
