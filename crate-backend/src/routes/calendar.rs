use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    calendar::{CalendarEvent, CalendarEventCreate, CalendarEventPatch},
    pagination::PaginationQuery,
    CalendarEventId, ThreadId, UserId,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    ServerState,
};

use super::util::Auth;

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct CalendarEventListQuery {
    from: Option<CalendarEventId>,
    to: Option<CalendarEventId>,
    limit: Option<u32>,
    from_time: Option<String>,
    to_time: Option<String>,
}

/// Calendar event list all
///
/// List all events the current user can see
#[utoipa::path(
    get,
    path = "/calendar/event",
    tags = ["calendar"],
    params(CalendarEventListQuery),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_list(
    Query(_query): Query<CalendarEventListQuery>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event list
#[utoipa::path(
    get,
    path = "/calendar/{thread_id}/event",
    tags = ["calendar"],
    params(("thread_id" = ThreadId, description = "Thread id")),
    responses((status = OK, description = "ok"))
)]
async fn calendar_thread_event_list(
    Path(_thread_id): Path<ThreadId>,
    Query(_query): Query<PaginationQuery<CalendarEventId>>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event create
#[utoipa::path(
    post,
    path = "/calendar/{thread_id}/event",
    tags = ["calendar"],
    params(("thread_id" = ThreadId, description = "Thread id")),
    request_body = CalendarEventCreate,
    responses((status = CREATED, body = CalendarEvent, description = "Create calendar event success"))
)]
async fn calendar_thread_event_create(
    Path(_thread_id): Path<ThreadId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<CalendarEventCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event get
#[utoipa::path(
    get,
    path = "/calendar/{thread_id}/event/{calendar_event_id}",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, body = CalendarEvent, description = "Get calendar event success"))
)]
async fn calendar_thread_event_get(
    Path((_thread_id, _calendar_event_id)): Path<(ThreadId, CalendarEventId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event update
#[utoipa::path(
    patch,
    path = "/calendar/{thread_id}/event/{calendar_event_id}",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    request_body = CalendarEventPatch,
    responses((status = OK, body = CalendarEvent, description = "Update calendar event success"))
)]
async fn calendar_thread_event_update(
    Path((_thread_id, _calendar_event_id)): Path<(ThreadId, CalendarEventId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<CalendarEventPatch>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event delete
#[utoipa::path(
    delete,
    path = "/calendar/{thread_id}/event/{calendar_event_id}",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = NO_CONTENT, description = "Delete calendar event success"))
)]
async fn calendar_thread_event_delete(
    Path((_thread_id, _calendar_event_id)): Path<(ThreadId, CalendarEventId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar rsvp list
#[utoipa::path(
    get,
    path = "/calendar/{thread_id}/event/{calendar_event_id}/rsvp",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_thread_event_rsvp_list(
    Path((_thread_id, _calendar_event_id)): Path<(ThreadId, CalendarEventId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar rsvp get
#[utoipa::path(
    get,
    path = "/calendar/{thread_id}/event/{calendar_event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserId, description = "User id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_thread_event_rsvp_get(
    Path((_thread_id, _calendar_event_id, _user_id)): Path<(ThreadId, CalendarEventId, UserId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar rsvp create
#[utoipa::path(
    put,
    path = "/calendar/{thread_id}/event/{calendar_event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserId, description = "User id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_thread_event_rsvp_update(
    Path((_thread_id, _calendar_event_id, _user_id)): Path<(ThreadId, CalendarEventId, UserId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar rsvp delete
#[utoipa::path(
    delete,
    path = "/calendar/{thread_id}/event/{calendar_event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserId, description = "User id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_thread_event_rsvp_delete(
    Path((_thread_id, _calendar_event_id, _user_id)): Path<(ThreadId, CalendarEventId, UserId)>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(calendar_event_list))
        .routes(routes!(calendar_thread_event_list))
        .routes(routes!(calendar_thread_event_create))
        .routes(routes!(calendar_thread_event_get))
        .routes(routes!(calendar_thread_event_update))
        .routes(routes!(calendar_thread_event_delete))
        .routes(routes!(calendar_thread_event_rsvp_list))
        .routes(routes!(calendar_thread_event_rsvp_get))
        .routes(routes!(calendar_thread_event_rsvp_update))
        .routes(routes!(calendar_thread_event_rsvp_delete))
}
