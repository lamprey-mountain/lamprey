use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{MessageId, MessageThreadUpdate, ThreadType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    types::{
        DbMessageCreate, DbThreadCreate, DbThreadType, MessageSync, MessageType, MessageVerId,
        PaginationQuery, PaginationResponse, Permission, RoomId, Thread, ThreadCreate, ThreadId,
        ThreadPatch,
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
        ThreadType::Forum => {
            perms.ensure(Permission::ThreadCreateForumTree)?;
        }
        ThreadType::Voice => {
            perms.ensure(Permission::ThreadCreateVoice)?;
        }
        _ => todo!(),
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
                _ => todo!(),
            },
        })
        .await?;
    let starter_message_id = data
        .message_create(DbMessageCreate {
            thread_id,
            attachment_ids: vec![],
            author_id: user_id,
            embeds: vec![],
            message_type: MessageType::ThreadUpdate(MessageThreadUpdate {
                patch: ThreadPatch {
                    name: Some(json.name),
                    description: Some(json.description),
                    tags: None,
                },
            }),
            edited_at: None,
            created_at: None,
        })
        .await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    let starter_message = data.message_get(thread_id, starter_message_id).await?;
    s.broadcast_room(
        room_id,
        user_id,
        reason,
        MessageSync::ThreadCreate {
            thread: thread.clone(),
        },
    )
    .await?;
    s.broadcast_thread(
        thread.id,
        user_id,
        None,
        MessageSync::MessageCreate {
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
    let mut res = dbg!(data.thread_list(room_id, q).await?);
    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        // FIXME: dubious performance
        threads.push(srv.threads.get(t.id, Some(user_id)).await?);
    }
    res.items = threads;
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
#[utoipa::path(
    put,
    path = "/room/{room_id}/pin/{thread_id}",
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
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

/// Unpin thread
#[utoipa::path(
    delete,
    path = "/room/{room_id}/pin/{thread_id}",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, body = Thread, description = "success"),
        (status = NOT_MODIFIED, body = Thread, description = "didn't change anything"),
    )
)]
async fn thread_unpin(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

/// Archive thread
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
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    if user_id != thread.creator_id {
        perms.ensure(Permission::ThreadArchive)?;
    }
    data.thread_archive(thread_id, user_id).await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        s.broadcast_room(
            room_id,
            user_id,
            reason,
            MessageSync::ThreadUpdate {
                thread: thread.clone(),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Unarchive thread
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
    let data = s.data();
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    if user_id != thread.creator_id {
        perms.ensure(Permission::ThreadArchive)?;
    }
    data.thread_unarchive(thread_id, user_id).await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        s.broadcast_room(
            room_id,
            user_id,
            reason,
            MessageSync::ThreadUpdate {
                thread: thread.clone(),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Delete thread
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
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure(Permission::ThreadDelete)?;
    data.thread_delete(thread_id, user_id).await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        s.broadcast_room(
            room_id,
            user_id,
            reason,
            MessageSync::ThreadUpdate {
                thread: thread.clone(),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Undelete thread
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/undelete",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn thread_undelete(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure(Permission::ThreadDelete)?;
    data.thread_undelete(thread_id, user_id).await?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    if let Some(room_id) = thread.room_id {
        s.broadcast_room(
            room_id,
            user_id,
            reason,
            MessageSync::ThreadUpdate {
                thread: thread.clone(),
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Send typing
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
        MessageSync::ThreadTyping {
            thread_id,
            user_id,
            until: until.into(),
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
        .routes(routes!(thread_unpin))
        .routes(routes!(thread_archive))
        .routes(routes!(thread_unarchive))
        .routes(routes!(thread_delete))
        .routes(routes!(thread_undelete))
        .routes(routes!(thread_typing))
}
