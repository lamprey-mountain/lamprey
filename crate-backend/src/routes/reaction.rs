use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::reaction::{ReactionKey, ReactionListItem};
use common::v1::types::{
    MessageId, MessageSync, PaginationQuery, PaginationResponse, Permission, ThreadId, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};
use crate::error::Result;
use crate::ServerState;

/// Message reaction add
///
/// Add a reaction to a message.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = CREATED, description = "new reaction created"),
        (status = OK, description = "already exists"),
    )
)]
async fn reaction_message_add(
    Path((thread_id, message_id, key)): Path<(ThreadId, MessageId, ReactionKey)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionAdd)?;
    data.reaction_message_put(auth_user_id, thread_id, message_id, key.clone())
        .await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        reason,
        MessageSync::ReactionMessageUpsert {
            thread_id,
            user_id: auth_user_id,
            message_id,
            key,
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

/// Message reaction remove
///
/// Remove a reaction from a message.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_message_remove(
    Path((thread_id, message_id, key)): Path<(ThreadId, MessageId, ReactionKey)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionAdd)?;
    data.reaction_message_delete(auth_user_id, thread_id, message_id, key.clone())
        .await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        reason,
        MessageSync::ReactionMessageRemove {
            thread_id,
            user_id: auth_user_id,
            message_id,
            key,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Message reaction purge
///
/// Remove all reactions from a message.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}/reaction",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["reaction"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_message_purge(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionClear)?;
    data.reaction_message_purge(thread_id, message_id).await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        reason,
        MessageSync::ReactionMessagePurge {
            thread_id,
            message_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Message reaction list
///
/// List message reactions for a specific emoji.
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
    params(
        PaginationQuery<UserId>,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = OK, body = PaginationResponse<ReactionListItem>, description = "success"),
    )
)]
async fn reaction_message_list(
    Path((thread_id, message_id, key)): Path<(ThreadId, MessageId, ReactionKey)>,
    Auth(auth_user_id): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    let list = data
        .reaction_message_list(thread_id, message_id, key, q)
        .await?;
    Ok(Json(list))
}

/// Thread reaction add
///
/// Add a reaction to a thread.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/reaction/{key}",
    params(
        ("thread_id", description = "Thread id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = CREATED, description = "new reaction created"),
        (status = NOT_MODIFIED, description = "already exists"),
    )
)]
async fn reaction_thread_add(
    Path((thread_id, key)): Path<(ThreadId, ReactionKey)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionAdd)?;
    data.reaction_thread_put(auth_user_id, thread_id, key.clone())
        .await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        reason,
        MessageSync::ReactionThreadUpsert {
            thread_id,
            user_id: auth_user_id,
            key,
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

/// Thread reaction remove
///
/// Remove a reaction from a thread.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/reaction/{key}",
    params(
        ("thread_id", description = "Thread id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_thread_remove(
    Path((thread_id, key)): Path<(ThreadId, ReactionKey)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionAdd)?;
    data.reaction_thread_delete(auth_user_id, thread_id, key.clone())
        .await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        reason,
        MessageSync::ReactionThreadRemove {
            thread_id,
            user_id: auth_user_id,
            key,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Thread reaction purge
///
/// Remove all reactions from a thread.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/reaction",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["reaction"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_thread_purge(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionClear)?;
    data.reaction_thread_purge(thread_id).await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        reason,
        MessageSync::ReactionThreadPurge { thread_id },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Thread reaction list
///
/// List thread reactions for a specific emoji.
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/reaction/{key}",
    params(
        PaginationQuery<UserId>,
        ("thread_id", description = "Thread id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = OK, body = PaginationResponse<ReactionListItem>, description = "success"),
    )
)]
async fn reaction_thread_list(
    Path((thread_id, key)): Path<(ThreadId, ReactionKey)>,
    Auth(auth_user_id): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    let list = data.reaction_thread_list(thread_id, key, q).await?;
    Ok(Json(list))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(reaction_message_add))
        .routes(routes!(reaction_message_remove))
        .routes(routes!(reaction_message_purge))
        .routes(routes!(reaction_message_list))
    // TODO: remove? you can react to the first message in the thread
    // anyways. the only reason to keep this is if there's other thread
    // types that can't be reacted to by default, which is likely to happen
    // to be fair.
    // .routes(routes!(reaction_thread_add))
    // .routes(routes!(reaction_thread_remove))
    // .routes(routes!(reaction_thread_purge))
    // .routes(routes!(reaction_thread_list))
}
