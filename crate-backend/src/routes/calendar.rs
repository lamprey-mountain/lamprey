use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    calendar::{
        CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventPatch,
        CalendarOverwrite, CalendarOverwritePut,
    },
    misc::UserIdReq,
    permission::Permission,
    CalendarEventId, ChannelId, UserId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::{Error, Result},
    routes::util::{Auth2, HeaderReason},
    ServerState,
};
use common::v1::types::{util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType};

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
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Calendar event list
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event",
    tags = ["calendar"],
    params(("channel_id" = ChannelId, description = "Channel id")),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_list(
    Path(channel_id): Path<ChannelId>,
    Query(query): Query<CalendarEventListQuery>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    query.validate()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let events = s.data().calendar_event_list(channel_id, query).await?;

    Ok(Json(events))
}

/// Calendar event create
#[utoipa::path(
    post,
    path = "/calendar/{channel_id}/event",
    tags = ["calendar"],
    params(("channel_id" = ChannelId, description = "Channel id")),
    request_body = CalendarEventCreate,
    responses((status = CREATED, body = CalendarEvent, description = "Create calendar event success"))
)]
async fn calendar_event_create(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<CalendarEventCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::CalendarEventCreate)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s
        .data()
        .calendar_event_create(json.clone(), channel_id, auth.user.id)
        .await?;

    let room_id = srv
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: reason.clone(),
        ty: AuditLogEntryType::CalendarEventCreate {
            changes: Changes::new()
                .add("title", &event.title)
                .add("description", &event.description)
                .add("location", &event.location)
                .add("starts_at", &event.starts_at)
                .add("ends_at", &event.ends_at)
                .build(),
        },
    })
    .await?;

    Ok((StatusCode::CREATED, Json(event)))
}

/// Calendar event get
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, body = CalendarEvent, description = "Get calendar event success"))
)]
async fn calendar_event_get(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;

    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    Ok(Json(event))
}

/// Calendar event update
#[utoipa::path(
    patch,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    request_body = CalendarEventPatch,
    responses((status = OK, body = CalendarEvent, description = "Update calendar event success"))
)]
async fn calendar_event_update(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<CalendarEventPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let old_event = s.data().calendar_event_get(event_id).await?;
    if old_event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    if old_event.creator_id == Some(auth.user.id) {
        perms.ensure(Permission::CalendarEventCreate)?;
    } else {
        perms.ensure(Permission::CalendarEventManage)?;
    }

    let updated_event = s
        .data()
        .calendar_event_update(event_id, json.clone())
        .await?;

    let room_id = srv
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
                .change("starts_at", &old_event.starts_at, &updated_event.starts_at)
                .change("ends_at", &old_event.ends_at, &updated_event.ends_at)
                .build(),
        },
    })
    .await?;

    Ok(Json(updated_event))
}

/// Calendar event delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = NO_CONTENT, description = "Delete calendar event success"))
)]
async fn calendar_event_delete(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }
    if event.creator_id == Some(auth.user.id) {
        perms.ensure(Permission::CalendarEventCreate)?;
    } else {
        perms.ensure(Permission::CalendarEventManage)?;
    }

    s.data().calendar_event_delete(event_id).await?;

    let room_id = srv
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: reason.clone(),
        ty: AuditLogEntryType::CalendarEventDelete {
            event_id: event.id,
            changes: Changes::new()
                .remove("title", &event.title)
                .remove("description", &event.description)
                .remove("location", &event.location)
                .remove("starts_at", &event.starts_at)
                .remove("ends_at", &event.ends_at)
                .build(),
        },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Calendar overwrite list
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, body = Vec<CalendarOverwrite>, description = "List calendar overwrites success"))
)]
async fn calendar_overwrite_list(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }
    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let overwrites = s.data().calendar_overwrite_list(event_id).await?;
    Ok(Json(overwrites))
}

/// Calendar overwrite get
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number")
    ),
    responses((status = OK, body = CalendarOverwrite, description = "Get calendar overwrite success"))
)]
async fn calendar_overwrite_get(
    Path((channel_id, event_id, seq)): Path<(ChannelId, CalendarEventId, u64)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }
    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let overwrite = s.data().calendar_overwrite_get(event_id, seq).await?;
    Ok(Json(overwrite))
}

/// Calendar overwrite update
#[utoipa::path(
    patch,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number")
    ),
    request_body = CalendarOverwritePut,
    responses((status = OK, body = CalendarOverwrite, description = "Update calendar overwrite success"))
)]
async fn calendar_overwrite_update(
    Path((channel_id, event_id, seq)): Path<(ChannelId, CalendarEventId, u64)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CalendarOverwritePut>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::CalendarEventManage)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }
    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let overwrite = s
        .data()
        .calendar_overwrite_put(event_id, seq, json)
        .await?;
    Ok(Json(overwrite))
}

/// Calendar overwrite delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number")
    ),
    responses((status = NO_CONTENT, description = "Delete calendar overwrite success"))
)]
async fn calendar_overwrite_delete(
    Path((channel_id, event_id, seq)): Path<(ChannelId, CalendarEventId, u64)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::CalendarEventManage)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }
    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data().calendar_overwrite_delete(event_id, seq).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Calendar Event RSVP list
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = OK, body = Vec<UserId>, description = "ok"))
)]
async fn calendar_event_rsvp_list(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let rsvps = s.data().calendar_event_rsvp_list(event_id).await?;
    Ok(Json(rsvps))
}

/// Calendar Event RSVP get
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserIdReq, description = "@self or user id"),
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_rsvp_get(
    Path((channel_id, event_id, user_id_req)): Path<(ChannelId, CalendarEventId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let rsvps = s.data().calendar_event_rsvp_list(event_id).await?;
    if rsvps.contains(&user_id) {
        Ok(StatusCode::OK)
    } else {
        Err(Error::NotFound)
    }
}

/// Calendar Event RSVP create
#[utoipa::path(
    put,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = UserIdReq, description = "@self or user id"),
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_rsvp_put(
    Path((channel_id, event_id, user_id_req)): Path<(ChannelId, CalendarEventId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    if auth.user.id != user_id {
        return Err(Error::BadStatic("cannot rsvp other people"));
    }

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data()
        .calendar_event_rsvp_put(event_id, user_id)
        .await?;

    Ok(StatusCode::OK)
}

/// Calendar Event RSVP delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = inline(UserIdReq), description = "@self or user id"),
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_rsvp_delete(
    Path((channel_id, event_id, user_id_req)): Path<(ChannelId, CalendarEventId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    if auth.user.id != user_id {
        perms.ensure(Permission::CalendarEventManage)?;
    }

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data()
        .calendar_event_rsvp_delete(event_id, user_id)
        .await?;

    Ok(StatusCode::OK)
}

/// Calendar Overwrite RSVP list
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}/rsvp",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number")
    ),
    responses((status = OK, body = Vec<(UserId, bool)>, description = "ok"))
)]
async fn calendar_overwrite_rsvp_list(
    Path((channel_id, event_id, seq)): Path<(ChannelId, CalendarEventId, u64)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let rsvps = s
        .data()
        .calendar_overwrite_rsvp_list(event_id, seq)
        .await?;
    Ok(Json(rsvps))
}

/// Calendar Overwrite RSVP create
#[utoipa::path(
    put,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number"),
        ("user_id" = UserIdReq, description = "@self or user id"),
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_overwrite_rsvp_put(
    Path((channel_id, event_id, seq, user_id_req)): Path<(
        ChannelId,
        CalendarEventId,
        u64,
        UserIdReq,
    )>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    if auth.user.id != user_id {
        return Err(Error::BadStatic("cannot rsvp other people"));
    }

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data()
        .calendar_overwrite_rsvp_put(event_id, seq, user_id, true)
        .await?;

    Ok(StatusCode::OK)
}

/// Calendar Overwrite RSVP delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}/rsvp/{user_id}",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number"),
        ("user_id" = inline(UserIdReq), description = "@self or user id"),
    ),
    responses((status = OK, description = "ok"))
)]
async fn calendar_overwrite_rsvp_delete(
    Path((channel_id, event_id, seq, user_id_req)): Path<(
        ChannelId,
        CalendarEventId,
        u64,
        UserIdReq,
    )>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    if auth.user.id != user_id {
        perms.ensure(Permission::CalendarEventManage)?;
    }

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }

    let event = s.data().calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    s.data()
        .calendar_overwrite_rsvp_delete(event_id, seq, user_id)
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
        .routes(routes!(calendar_overwrite_list))
        .routes(routes!(calendar_overwrite_get))
        .routes(routes!(calendar_overwrite_update))
        .routes(routes!(calendar_overwrite_delete))
        .routes(routes!(calendar_event_rsvp_list))
        .routes(routes!(calendar_event_rsvp_get))
        .routes(routes!(calendar_event_rsvp_put))
        .routes(routes!(calendar_event_rsvp_delete))
        .routes(routes!(calendar_overwrite_rsvp_list))
        .routes(routes!(calendar_overwrite_rsvp_put))
        .routes(routes!(calendar_overwrite_rsvp_delete))
}