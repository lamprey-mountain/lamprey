use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use types::{MessageId, PaginationQuery, PaginationResponse, ThreadId, UserId};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReactionListItem {
    pub user_id: UserId,
}

// FIXME: redesign routes

/// Message reaction add (TODO)
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
    Path((_thread_id, _message_id, _key)): Path<(ThreadId, MessageId, String)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Message reaction remove (TODO)
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
    Path((_thread_id, _message_id, _key)): Path<(ThreadId, MessageId, String)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

// /// Message reaction get (TODO)
// ///
// /// Get a reaction from a message.
// #[utoipa::path(
//     get,
//     path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
//     params(
//         ("thread_id", description = "Thread id"),
//         ("message_id", description = "Message id"),
//         ("key", description = "Reaction key"),
//     ),
//     tags = ["reaction"],
//     responses(
//         (status = OK, description = "reaction exists"),
//         (status = NOT_FOUND, description = "reaction doesnt exist"),
//     )
// )]
// async fn reaction_message_get(
//     Path((_thread_id, _message_id, _key)): Path<(ThreadId, MessageId, String)>,
//     Auth(_auth_user_id): Auth,
//     State(_s): State<Arc<ServerState>>,
// ) -> Result<Json<()>> {
//     Err(Error::Unimplemented)
// }

/// Message reaction list (TODO)
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
    Path((_thread_id, _message_id, _key)): Path<(ThreadId, MessageId, String)>,
    Auth(_auth_user_id): Auth,
    Query(_q): Query<PaginationQuery<UserId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Thread reaction add (TODO)
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
    Path((_thread_id, _key)): Path<(ThreadId, String)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Thread reaction remove (TODO)
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
    Path((_thread_id, _key)): Path<(ThreadId, String)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

// /// Thread reaction get (TODO)
// ///
// /// Get a reaction from a thread.
// #[utoipa::path(
//     get,
//     path = "/thread/{thread_id}/reaction/{key}",
//     params(
//         ("thread_id", description = "Thread id"),
//         ("key", description = "Reaction key"),
//     ),
//     tags = ["reaction"],
//     responses(
//         (status = OK, description = "reaction exists"),
//         (status = NOT_FOUND, description = "reaction doesnt exist"),
//     )
// )]
// async fn reaction_thread_get(
//     Path((_thread_id, _key)): Path<(ThreadId, String)>,
//     Auth(_auth_user_id): Auth,
//     State(_s): State<Arc<ServerState>>,
// ) -> Result<Json<()>> {
//     Err(Error::Unimplemented)
// }

/// Thread reaction list (TODO)
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
    Path((_thread_id, _key)): Path<(ThreadId, String)>,
    Auth(_auth_user_id): Auth,
    Query(_q): Query<PaginationQuery<UserId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(reaction_message_add))
        // .routes(routes!(reaction_message_get))
        .routes(routes!(reaction_message_remove))
        .routes(routes!(reaction_message_list))
        .routes(routes!(reaction_thread_add))
        // .routes(routes!(reaction_thread_get))
        .routes(routes!(reaction_thread_remove))
        .routes(routes!(reaction_thread_list))
}
