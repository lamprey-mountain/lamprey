use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    calendar::{CalendarEvent, CalendarEventCreate, CalendarEventPatch},
    pagination::PaginationQuery,
    permission::Permission,
    CalendarEventId, ChannelId, UserId,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::{Error, Result},
    routes::util::HeaderReason,
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

/// Calendar event list user (TODO)
///
/// List all events the current user can see
#[utoipa::path(
    get,
    path = "/calendar/event",
    tags = ["calendar"],
    params(CalendarEventListQuery),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_list_user(
    Query(_query): Query<CalendarEventListQuery>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event list
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/event",
    tags = ["calendar"],
    params(("channel_id" = ChannelId, description = "Channel id")),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_list(
    Path(channel_id): Path<ChannelId>,
    Query(query): Query<PaginationQuery<CalendarEventId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;

    let events = s.data().calendar_event_list(channel_id, query).await?;

    Ok(Json(events))
}

/// Calendar event create
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/event",
    tags = ["calendar"],
    params(("channel_id" = ChannelId, description = "Channel id")),
    request_body = CalendarEventCreate,
    responses((status = CREATED, body = CalendarEvent, description = "Create calendar event success"))
)]
async fn calendar_event_create(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CalendarEventCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::CalendarEventManage)?;

    let event = s
        .data()
        .calendar_event_create(json, channel_id, auth_user.id)
        .await?;

    Ok((StatusCode::CREATED, Json(event)))
}

/// Calendar event get
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/event/{calendar_event_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, body = CalendarEvent, description = "Get calendar event success"))
)]
async fn calendar_event_get(
    Path((channel_id, calendar_event_id)): Path<(ChannelId, CalendarEventId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;

    let event = s.data().calendar_event_get(calendar_event_id).await?;

    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    Ok(Json(event))
}

/// Calendar event update
#[utoipa::path(
    patch,
    path = "/channel/{channel_id}/event/{calendar_event_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    request_body = CalendarEventPatch,
    responses((status = OK, body = CalendarEvent, description = "Update calendar event success"))
)]
async fn calendar_channel_event_update(
    Path((channel_id, calendar_event_id)): Path<(ChannelId, CalendarEventId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CalendarEventPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::CalendarEventManage)?;

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let updated_event = s
        .data()
        .calendar_event_update(calendar_event_id, json)
        .await?;

    Ok(Json(updated_event))
}

/// Calendar event delete
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/event/{calendar_event_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = NO_CONTENT, description = "Delete calendar event success"))
)]
async fn calendar_event_delete(
    Path((channel_id, calendar_event_id)): Path<(ChannelId, CalendarEventId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::CalendarEventManage)?;

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data().calendar_event_delete(calendar_event_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Calendar rsvp list
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/event/{calendar_event_id}/rsvp",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, body = Vec<UserId>, description = "ok"))
)]
async fn calendar_rsvp_list(
    Path((channel_id, calendar_event_id)): Path<(ChannelId, CalendarEventId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let rsvps = s.data().calendar_event_rsvp_list(calendar_event_id).await?;
    Ok(Json(rsvps))
}

/// Calendar rsvp get
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/event/{calendar_event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserId, description = "User id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_rsvp_get(
    Path((channel_id, calendar_event_id, user_id)): Path<(ChannelId, CalendarEventId, UserId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let rsvps = s.data().calendar_event_rsvp_list(calendar_event_id).await?;
    if rsvps.contains(&user_id) {
        Ok(StatusCode::OK)
    } else {
        Err(Error::NotFound)
    }
}

/// Calendar rsvp create
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/event/{calendar_event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserId, description = "User id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_rsvp_update(
    Path((channel_id, calendar_event_id, user_id)): Path<(ChannelId, CalendarEventId, UserId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let _perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data()
        .calendar_event_rsvp_put(calendar_event_id, user_id)
        .await?;

    Ok(StatusCode::OK)
}

/// Calendar rsvp delete
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/event/{calendar_event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("calendar_event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserId, description = "User id")
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_rsvp_delete(
    Path((channel_id, calendar_event_id, user_id)): Path<(ChannelId, CalendarEventId, UserId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if auth_user.id != user_id {
        let perms = s
            .services()
            .perms
            .for_channel(auth_user.id, channel_id)
            .await?;
        perms.ensure(Permission::CalendarEventManage)?;
    }

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data()
        .calendar_event_rsvp_delete(calendar_event_id, user_id)
        .await?;

    Ok(StatusCode::OK)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(calendar_event_list_user))
        .routes(routes!(calendar_event_list))
        .routes(routes!(calendar_event_create))
        .routes(routes!(calendar_event_get))
        .routes(routes!(calendar_channel_event_update))
        .routes(routes!(calendar_event_delete))
        .routes(routes!(calendar_rsvp_list))
        .routes(routes!(calendar_rsvp_get))
        .routes(routes!(calendar_rsvp_update))
        .routes(routes!(calendar_rsvp_delete))
}
