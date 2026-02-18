use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    calendar::{
        CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventParticipant,
        CalendarEventParticipantPut, CalendarEventParticipantQuery, CalendarEventPatch,
        CalendarOverwrite, CalendarOverwritePut, CalendarRsvpStatus,
    },
    misc::UserIdReq,
    permission::Permission,
    sync::MessageSync,
    CalendarEventId, ChannelId, UserId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::{Error, Result},
    routes::util::Auth,
    ServerState,
};
use common::v1::types::{util::Changes, AuditLogEntryType};

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
    _auth: Auth,
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
    auth: Auth,
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
    tags = ["calendar", "badge.audit-log.CalendarEventCreate"],
    params(("channel_id" = ChannelId, description = "Channel id")),
    request_body = CalendarEventCreate,
    responses((status = CREATED, body = CalendarEvent, description = "Create calendar event success"))
)]
async fn calendar_event_create(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
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

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::CalendarEventCreate {
        changes: Changes::new()
            .add("title", &event.title)
            .add("description", &event.description)
            .add("location", &event.location)
            .add("starts_at", &event.starts_at)
            .add("ends_at", &event.ends_at)
            .build(),
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarEventCreate {
            event: event.clone(),
        },
    )
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
    auth: Auth,
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
    tags = ["calendar", "badge.audit-log.CalendarEventUpdate"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    request_body = CalendarEventPatch,
    responses((status = OK, body = CalendarEvent, description = "Update calendar event success"))
)]
async fn calendar_event_update(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
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

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::CalendarEventUpdate {
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
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarEventUpdate {
            event: updated_event.clone(),
        },
    )
    .await?;

    Ok(Json(updated_event))
}

/// Calendar event delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}",
    tags = ["calendar", "badge.audit-log.CalendarEventDelete"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id")
    ),
    responses((status = NO_CONTENT, description = "Delete calendar event success"))
)]
async fn calendar_event_delete(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    auth: Auth,
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

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::CalendarEventDelete {
        event_id: event.id,
        changes: Changes::new()
            .remove("title", &event.title)
            .remove("description", &event.description)
            .remove("location", &event.location)
            .remove("starts_at", &event.starts_at)
            .remove("ends_at", &event.ends_at)
            .build(),
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarEventDelete {
            channel_id,
            event_id: event.id,
        },
    )
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
    auth: Auth,
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
    auth: Auth,
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
    tags = ["calendar", "badge.audit-log.CalendarOverwriteUpdate", "badge.audit-log.CalendarOverwriteCreate"],
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CalendarOverwritePut>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::CalendarEventManage)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(Error::BadStatic("channel is not a calendar"));
    }
    let event = data.calendar_event_get(event_id).await?;
    if event.channel_id != channel_id {
        return Err(Error::NotFound);
    }

    let old_overwrite = data.calendar_overwrite_get(event_id, seq).await.ok();

    let overwrite = data.calendar_overwrite_put(event_id, seq, json).await?;

    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let sync_event;
    if let Some(old) = old_overwrite {
        sync_event = MessageSync::CalendarOverwriteUpdate {
            channel_id,
            overwrite: overwrite.clone(),
        };

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarOverwriteUpdate {
            event_id,
            seq,
            changes: Changes::new()
                .change("title", &old.title, &overwrite.title)
                .change(
                    "extra_description",
                    &old.extra_description,
                    &overwrite.extra_description,
                )
                .change("location", &old.location, &overwrite.location)
                .change("url", &old.url, &overwrite.url)
                .change("starts_at", &old.starts_at, &overwrite.starts_at)
                .change("ends_at", &old.ends_at, &overwrite.ends_at)
                .change("cancelled", &old.cancelled, &overwrite.cancelled)
                .build(),
        })
        .await?;
    } else {
        sync_event = MessageSync::CalendarOverwriteCreate {
            channel_id,
            overwrite: overwrite.clone(),
        };

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarOverwriteCreate {
            event_id,
            seq,
            changes: Changes::new()
                .add("title", &overwrite.title)
                .add("extra_description", &overwrite.extra_description)
                .add("location", &overwrite.location)
                .add("url", &overwrite.url)
                .add("starts_at", &overwrite.starts_at)
                .add("ends_at", &overwrite.ends_at)
                .add("cancelled", &overwrite.cancelled)
                .build(),
        })
        .await?;
    }

    s.broadcast_room(room_id, auth.user.id, sync_event).await?;

    Ok(Json(overwrite))
}

/// Calendar overwrite delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}",
    tags = ["calendar", "badge.audit-log.CalendarOverwriteDelete"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number")
    ),
    responses((status = NO_CONTENT, description = "Delete calendar overwrite success"))
)]
async fn calendar_overwrite_delete(
    Path((channel_id, event_id, seq)): Path<(ChannelId, CalendarEventId, u64)>,
    auth: Auth,
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

    let overwrite = s.data().calendar_overwrite_get(event_id, seq).await?;
    s.data().calendar_overwrite_delete(event_id, seq).await?;

    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::CalendarOverwriteDelete {
        event_id,
        seq,
        changes: Changes::new()
            .remove("title", &overwrite.title)
            .remove("extra_description", &overwrite.extra_description)
            .remove("location", &overwrite.location)
            .remove("url", &overwrite.url)
            .remove("starts_at", &overwrite.starts_at)
            .remove("ends_at", &overwrite.ends_at)
            .remove("cancelled", &overwrite.cancelled)
            .build(),
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarOverwriteDelete {
            channel_id,
            event_id,
            seq,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

use std::collections::HashMap;

/// Calendar Event RSVP list
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        CalendarEventParticipantQuery
    ),
    responses((status = OK, body = Vec<CalendarEventParticipant>, description = "ok"))
)]
async fn calendar_event_rsvp_list(
    Path((channel_id, event_id)): Path<(ChannelId, CalendarEventId)>,
    Query(query): Query<CalendarEventParticipantQuery>,
    auth: Auth,
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

    let mut participants = s
        .data()
        .calendar_event_rsvp_list(event_id, query.clone())
        .await?;

    if query.include_member && !participants.is_empty() {
        let user_ids: Vec<UserId> = participants.iter().map(|p| p.user_id).collect();
        // TODO: get_many users with user config
        let users = srv.users.get_many(&user_ids).await?;
        let mut users_map: HashMap<_, _> = users.into_iter().map(|u| (u.id, u)).collect();

        let mut members_map = HashMap::new();
        if let Some(room_id) = chan.room_id {
            // TODO: bulk fetch members
            for uid in &user_ids {
                if let Ok(member) = s.data().room_member_get(room_id, *uid).await {
                    members_map.insert(*uid, member);
                }
            }
        }

        for p in &mut participants {
            p.user = users_map.remove(&p.user_id);
            p.member = members_map.remove(&p.user_id);
        }
    }

    Ok(Json(participants))
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
    responses((status = OK, body = CalendarEventParticipant, description = "ok"))
)]
async fn calendar_event_rsvp_get(
    Path((channel_id, event_id, user_id_req)): Path<(ChannelId, CalendarEventId, UserIdReq)>,
    auth: Auth,
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

    let rsvp = s.data().calendar_event_rsvp_get(event_id, user_id).await?;
    Ok(Json(rsvp))
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
    request_body = CalendarEventParticipantPut,
    responses((status = OK, description = "ok"))
)]
async fn calendar_event_rsvp_put(
    Path((channel_id, event_id, user_id_req)): Path<(ChannelId, CalendarEventId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CalendarEventParticipantPut>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::CalendarEventRsvp)?;

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

    let room_id = srv
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    match json.status {
        CalendarRsvpStatus::Interested => {
            s.data().calendar_event_rsvp_put(event_id, user_id).await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarRsvpCreate {
                    channel_id,
                    event_id,
                    participant: CalendarEventParticipant {
                        user_id,
                        status: json.status,
                        user: None,
                        member: None,
                    },
                },
            )
            .await?;
        }
        CalendarRsvpStatus::Uninterested => {
            s.data()
                .calendar_event_rsvp_delete(event_id, user_id)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarRsvpDelete {
                    channel_id,
                    event_id,
                    user_id,
                },
            )
            .await?;
        }
    }

    Ok(StatusCode::OK)
}

/// Calendar Event RSVP delete
#[utoipa::path(
    delete,
    path = "/calendar/{channel_id}/event/{event_id}/rsvp/{user_id}",
    tags = ["calendar", "badge.audit-log.CalendarRsvpDelete"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("user_id" = inline(UserIdReq), description = "@self or user id"),
    ),
    responses((status = NO_CONTENT, description = "Delete calendar event RSVP success"))
)]
async fn calendar_event_rsvp_delete(
    Path((channel_id, event_id, user_id_req)): Path<(ChannelId, CalendarEventId, UserIdReq)>,
    auth: Auth,
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
    } else {
        perms.ensure(Permission::CalendarEventRsvp)?;
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

    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    if auth.user.id != user_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarRsvpDelete {
            event_id,
            seq: None,
            user_id,
        })
        .await?;
    }

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarRsvpDelete {
            channel_id,
            event_id,
            user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Calendar Overwrite RSVP list
#[utoipa::path(
    get,
    path = "/calendar/{channel_id}/event/{event_id}/overwrite/{seq}/rsvp",
    tags = ["calendar"],
    params(
        ("channel_id" = ChannelId, description = "Channel id"),
        ("event_id" = CalendarEventId, description = "Calendar event id"),
        ("seq" = u64, description = "Sequence number"),
        CalendarEventParticipantQuery
    ),
    responses((status = OK, body = Vec<CalendarEventParticipant>, description = "ok"))
)]
async fn calendar_overwrite_rsvp_list(
    Path((channel_id, event_id, seq)): Path<(ChannelId, CalendarEventId, u64)>,
    Query(query): Query<CalendarEventParticipantQuery>,
    auth: Auth,
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

    let mut participants = s
        .data()
        .calendar_overwrite_rsvp_list(event_id, seq, query.clone())
        .await?;

    if query.include_member && !participants.is_empty() {
        let user_ids: Vec<UserId> = participants.iter().map(|p| p.user_id).collect();
        // TODO: populate user_config, same as above
        let users = srv.users.get_many(&user_ids).await?;
        let mut users_map: HashMap<_, _> = users.into_iter().map(|u| (u.id, u)).collect();

        let mut members_map = HashMap::new();
        if let Some(room_id) = chan.room_id {
            // TODO: bulk fetch members
            for uid in &user_ids {
                if let Ok(member) = s.data().room_member_get(room_id, *uid).await {
                    members_map.insert(*uid, member);
                }
            }
        }

        for p in &mut participants {
            p.user = users_map.remove(&p.user_id);
            p.member = members_map.remove(&p.user_id);
        }
    }

    Ok(Json(participants))
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
    request_body = CalendarEventParticipantPut,
    responses((status = OK, description = "ok"))
)]
async fn calendar_overwrite_rsvp_put(
    Path((channel_id, event_id, seq, user_id_req)): Path<(
        ChannelId,
        CalendarEventId,
        u64,
        UserIdReq,
    )>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CalendarEventParticipantPut>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::CalendarEventRsvp)?;

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

    let room_id = srv
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    match json.status {
        CalendarRsvpStatus::Interested => {
            s.data()
                .calendar_overwrite_rsvp_put(event_id, seq, user_id, true)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarOverwriteRsvpCreate {
                    channel_id,
                    event_id,
                    seq,
                    participant: CalendarEventParticipant {
                        user_id,
                        status: json.status,
                        user: None,
                        member: None,
                    },
                },
            )
            .await?;
        }
        CalendarRsvpStatus::Uninterested => {
            s.data()
                .calendar_overwrite_rsvp_put(event_id, seq, user_id, false)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarOverwriteRsvpDelete {
                    channel_id,
                    event_id,
                    seq,
                    user_id,
                },
            )
            .await?;
        }
    }

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
    responses((status = NO_CONTENT, description = "Delete calendar overwrite RSVP success"))
)]
async fn calendar_overwrite_rsvp_delete(
    Path((channel_id, event_id, seq, user_id_req)): Path<(
        ChannelId,
        CalendarEventId,
        u64,
        UserIdReq,
    )>,
    auth: Auth,
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
    } else {
        perms.ensure(Permission::CalendarEventRsvp)?;
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

    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    if auth.user.id != user_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarRsvpDelete {
            event_id,
            seq: Some(seq),
            user_id,
        })
        .await?;
    }

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarOverwriteRsvpDelete {
            channel_id,
            event_id,
            seq,
            user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
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
