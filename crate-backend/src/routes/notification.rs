use std::sync::Arc;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::notifications::Notification;
use common::v1::types::{
    MessageId, NotificationId, PaginationQuery, PaginationResponse, RoomId, Thread, ThreadId,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use super::util::Auth;
use crate::error::Result;
use crate::{Error, ServerState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
pub struct InboxListParams {
    /// only include notifications from these rooms
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub room_id: Vec<RoomId>,

    /// only include notifications from these threads
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub thread_id: Vec<ThreadId>,

    /// include messages marked as read too
    #[serde(default)]
    pub include_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NotificationCreate {
    /// the thread this message was sent in
    pub thread_id: ThreadId,

    /// the id of the message that was sent
    pub message_id: MessageId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct NotificationMarkRead {
    /// mark these messages as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Vec<MessageId>,

    /// mark everything in these threads as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub thread_ids: Vec<ThreadId>,

    /// mark everything in these rooms as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub room_ids: Vec<RoomId>,

    /// mark everything as read
    #[serde(default)]
    pub everything: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct NotificationFlush {
    /// restrict to just notifications before (including) this message id
    pub before: Option<MessageId>,

    /// restrict to just notifications after (including) this message id
    pub after: Option<MessageId>,

    /// restrict to just these messages
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Option<Vec<MessageId>>,

    /// restrict to just these threads
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub thread_ids: Option<Vec<ThreadId>>,

    /// restrict to just these rooms
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub room_ids: Option<Vec<RoomId>>,

    /// also include unread notifications
    #[serde(default)]
    pub include_unread: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct InboxThreadsParams {
    /// the order to return inbox threads in
    pub order: InboxThreadsOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum InboxThreadsOrder {
    /// most active threads first (order by last_version_id desc)
    Activity,

    /// last active threads first (order by last_version_id asc)
    // NOTE: not sure how useful this is, but including for completeness
    Inactivity,

    /// most recently created threads first (order by id desc)
    Newest,

    /// most recently created threads first (order by id desc)
    Oldest,
}

/// Inbox get (TODO)
///
/// List notifications
#[utoipa::path(
    get,
    path = "/inbox",
    params(PaginationQuery<MessageId>, InboxListParams),
    tags = ["inbox"],
    responses((status = OK, body = PaginationResponse<Notification>, description = "success"))
)]
async fn inbox_get(
    Auth(_auth_user_id): Auth,
    Query(_pagination): Query<PaginationQuery<NotificationId>>,
    Query(_params): Query<InboxListParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox post (TODO)
///
/// Create a reminder for later
#[utoipa::path(
    post,
    path = "/inbox",
    tags = ["inbox"],
    responses((status = OK, body = Notification, description = "success"))
)]
async fn inbox_post(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox threads (TODO)
///
/// Get a list of all unread threads
#[utoipa::path(
    get,
    path = "/inbox/threads",
    tags = ["inbox"],
    params(PaginationQuery<ThreadId>, InboxListParams, InboxThreadsParams),
    responses((status = OK, body = PaginationResponse<Thread>, description = "success"))
)]
async fn inbox_threads(
    Auth(_auth_user_id): Auth,
    Query(_pagination): Query<PaginationQuery<ThreadId>>,
    Query(_inbox_params): Query<InboxListParams>,
    Query(_thread_params): Query<InboxThreadsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox mark read (TODO)
#[utoipa::path(
    post,
    path = "/inbox/mark-read",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_read(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox mark unread (TODO)
#[utoipa::path(
    post,
    path = "/inbox/mark-unread",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_unread(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox flush (TODO)
///
/// Deletes read notifications from the inbox
#[utoipa::path(
    post,
    path = "/inbox/flush",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_flush(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationFlush>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(inbox_get))
        .routes(routes!(inbox_post))
        .routes(routes!(inbox_threads))
        .routes(routes!(inbox_mark_read))
        .routes(routes!(inbox_mark_unread))
        .routes(routes!(inbox_flush))
}
