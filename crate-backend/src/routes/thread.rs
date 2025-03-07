use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use types::{MessageId, ThreadState, ThreadType};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    types::{
        DbMessageCreate, DbThreadCreate, MessageSync, MessageType, MessageVerId, PaginationQuery,
        PaginationResponse, Permission, RoomId, Thread, ThreadCreate, ThreadId, ThreadPatch,
    },
    ServerState,
};

use super::util::{Auth, HeaderReason};
use crate::error::Result;

/// Create a thread
#[utoipa::path(
    post,
    // path = "/thread",
    path = "/room/{room_id}/thread",
    params(("room_id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = CREATED, body = Thread, description = "Create thread success"),
    )
)]
async fn thread_create(
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
    };
    let thread_id = data
        .thread_create(DbThreadCreate {
            room_id,
            creator_id: user_id,
            name: json.name.clone(),
            description: json.description.clone(),
        })
        .await?;
    let starter_message_id = data
        .message_create(DbMessageCreate {
            thread_id,
            attachment_ids: vec![],
            author_id: user_id,
            message_type: MessageType::ThreadUpdate(types::MessageThreadUpdate {
                patch: ThreadPatch {
                    name: Some(json.name),
                    description: Some(json.description),
                    state: None,
                },
            }),
        })
        .await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    let starter_message = data.message_get(thread_id, starter_message_id).await?;
    s.broadcast_room(
        room_id,
        user_id,
        reason,
        MessageSync::UpsertThread {
            thread: thread.clone(),
        },
    )
    .await?;
    s.broadcast_thread(
        thread.id,
        user_id,
        None,
        MessageSync::UpsertMessage {
            message: starter_message,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(thread)))
}

/// Get a thread
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

/// List threads in a room
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
    let res = data.thread_list(user_id, room_id, q).await?;
    Ok(Json(res))
}

/// Edit a thread
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

/// Ack thread
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
        data.message_version_get(thread_id, version_id).await?.id
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

/// Pin thread
///
/// Set a thread's state to Pinned.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/pin",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_pin(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let patch = ThreadPatch {
        name: None,
        description: None,
        state: Some(ThreadState::Pinned { pin_order: 0 }),
    };
    let thread = s
        .services()
        .threads
        .update(user_id, thread_id, patch, reason)
        .await?;
    Ok(Json(thread))
}

/// Archive thread
///
/// Set a thread's state to Archived.
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
    let patch = ThreadPatch {
        name: None,
        description: None,
        state: Some(ThreadState::Archived),
    };
    let thread = s
        .services()
        .threads
        .update(user_id, thread_id, patch, reason)
        .await?;
    Ok(Json(thread))
}

/// Reopen/unpin thread
///
/// Set a thread's state to Default.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/activate",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_activate(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let patch = ThreadPatch {
        name: None,
        description: None,
        state: Some(ThreadState::Active),
    };
    let thread = s
        .services()
        .threads
        .update(user_id, thread_id, patch, reason)
        .await?;
    Ok(Json(thread))
}

/// Delete thread
///
/// Set a thread's state to Deleted.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn thread_delete(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let patch = ThreadPatch {
        name: None,
        description: None,
        state: Some(ThreadState::Deleted),
    };
    s.services()
        .threads
        .update(user_id, thread_id, patch, reason)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Send typing
///
/// Send a typing notification to a thread
#[utoipa::path(
    put,
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
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    s.broadcast_thread(
        thread_id,
        user_id,
        reason,
        MessageSync::Typing {
            thread_id,
            user_id,
            until,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_create))
        .routes(routes!(thread_get))
        .routes(routes!(thread_list))
        .routes(routes!(thread_update))
        .routes(routes!(thread_ack))
        .routes(routes!(thread_pin))
        .routes(routes!(thread_archive))
        .routes(routes!(thread_delete))
        .routes(routes!(thread_activate))
        .routes(routes!(thread_typing))
}
