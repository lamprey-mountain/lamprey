use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    calendar::{CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventPatch},
    permission::Permission,
    CalendarEventId, ChannelId, UserId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::{Error, Result},
    routes::util::HeaderReason,
    ServerState,
};
use common::v1::types::{util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType};

use super::util::Auth;

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
    Query(query): Query<CalendarEventListQuery>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    query.validate()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth_user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth_user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

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
    HeaderReason(reason): HeaderReason,
    Json(json): Json<CalendarEventCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::CalendarEventManage)?;

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s
        .data()
        .calendar_event_create(json.clone(), channel_id, auth_user.id)
        .await?;

    let room_id = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::CalendarEventCreate {
            changes: Changes::new()
                .add("title", &event.title)
                .add("description", &event.description)
                .add("location", &event.location)
                .add("start", &event.start)
                .add("end", &event.end)
                .build(),
        },
    })
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

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

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
async fn calendar_event_update(
    Path((channel_id, calendar_event_id)): Path<(ChannelId, CalendarEventId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<CalendarEventPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::CalendarEventManage)?;

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let old_event = s.data().calendar_event_get(calendar_event_id).await?;
    if old_event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let updated_event = s
        .data()
        .calendar_event_update(calendar_event_id, json.clone())
        .await?;

    let room_id = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::CalendarEventUpdate {
            changes: Changes::new()
                .change("title", &old_event.title, &updated_event.title)
                .change(
                    "description",
                    &old_event.description,
                    &updated_event.description,
                )
                .change("location", &old_event.location, &updated_event.location)
                .change("start", &old_event.start, &updated_event.start)
                .change("end", &old_event.end, &updated_event.end)
                .build(),
        },
    })
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
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth_user.id, channel_id)
        .await?;
    perms.ensure(Permission::CalendarEventManage)?;

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(calendar_event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data().calendar_event_delete(calendar_event_id).await?;

    let room_id = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::CalendarEventDelete {
            title: event.title,
            event_id: event.id,
        },
    })
    .await?;

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

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

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

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

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

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

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

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth_user.id))
        .await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
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
        .routes(routes!(calendar_event_update))
        .routes(routes!(calendar_event_delete))
        .routes(routes!(calendar_rsvp_list))
        .routes(routes!(calendar_rsvp_get))
        .routes(routes!(calendar_rsvp_update))
        .routes(routes!(calendar_rsvp_delete))
}
