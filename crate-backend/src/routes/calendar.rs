use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::{
    calendar::{CalendarEventParticipant, CalendarRsvpStatus},
    misc::UserIdReq,
    AuditLogEntryType, MessageSync, Permission, UserId,
};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Calendar event list user
#[handler(routes::calendar_event_list_user)]
async fn calendar_event_list_user(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::calendar_event_list_user::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    Ok(Error::Unimplemented)
}

/// Calendar event list
#[handler(routes::calendar_event_list)]
async fn calendar_event_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    req.query.validate()?;

    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let events = s
        .data()
        .calendar_event_list(req.channel_id, req.query)
        .await?;

    Ok(Json(events))
}

/// Calendar event create
#[handler(routes::calendar_event_create)]
async fn calendar_event_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    req.event.validate()?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::CalendarEventCreate)
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s
        .data()
        .calendar_event_create(req.event.clone(), req.channel_id, auth.user.id)
        .await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

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
#[handler(routes::calendar_event_get)]
async fn calendar_event_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    Ok(Json(event))
}

/// Calendar event update
#[handler(routes::calendar_event_update)]
async fn calendar_event_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    req.patch.validate()?;
    let srv = s.services();
    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let old_event = s.data().calendar_event_get(req.event_id).await?;
    if old_event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    if old_event.creator_id == Some(auth.user.id) {
        perms.needs(Permission::CalendarEventCreate);
    } else {
        perms.needs(Permission::CalendarEventManage);
    }
    perms.check()?;

    let updated_event = s
        .data()
        .calendar_event_update(req.event_id, req.patch.clone())
        .await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

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
#[handler(routes::calendar_event_delete)]
async fn calendar_event_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }
    if event.creator_id == Some(auth.user.id) {
        perms.needs(Permission::CalendarEventCreate);
    } else {
        perms.needs(Permission::CalendarEventManage);
    }
    perms.check()?;

    s.data().calendar_event_delete(req.event_id).await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

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
            channel_id: req.channel_id,
            event_id: event.id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Calendar event RSVP list
#[handler(routes::calendar_event_rsvp_list)]
async fn calendar_event_rsvp_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_rsvp_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let mut participants = s
        .data()
        .calendar_event_rsvp_list(req.event_id, req.query.clone())
        .await?;

    if req.query.include_member && !participants.is_empty() {
        let user_ids: Vec<UserId> = participants.iter().map(|p| p.user_id).collect();
        let users = srv.users.get_many(&user_ids).await?;
        let mut users_map: HashMap<_, _> = users.into_iter().map(|u| (u.id, u)).collect();

        let mut members_map = HashMap::new();
        if let Some(room_id) = chan.room_id {
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

/// Calendar event RSVP get
#[handler(routes::calendar_event_rsvp_get)]
async fn calendar_event_rsvp_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_rsvp_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let rsvp = s
        .data()
        .calendar_event_rsvp_get(req.event_id, user_id)
        .await?;
    Ok(Json(rsvp))
}

/// Calendar event RSVP put
#[handler(routes::calendar_event_rsvp_put)]
async fn calendar_event_rsvp_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_rsvp_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::CalendarEventRsvp)
        .check()?;

    if auth.user.id != user_id {
        return Err(ApiError::from_code(ErrorCode::CannotRsvpOtherPeople).into());
    }

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

    match req.participant.status {
        CalendarRsvpStatus::Interested => {
            s.data()
                .calendar_event_rsvp_put(req.event_id, user_id)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarRsvpCreate {
                    channel_id: req.channel_id,
                    event_id: req.event_id,
                    participant: CalendarEventParticipant {
                        user_id,
                        status: req.participant.status,
                        user: None,
                        member: None,
                    },
                },
            )
            .await?;
        }
        CalendarRsvpStatus::Uninterested => {
            s.data()
                .calendar_event_rsvp_delete(req.event_id, user_id)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarRsvpDelete {
                    channel_id: req.channel_id,
                    event_id: req.event_id,
                    user_id,
                },
            )
            .await?;
        }
    }

    Ok(StatusCode::OK)
}

/// Calendar event RSVP delete
#[handler(routes::calendar_event_rsvp_delete)]
async fn calendar_event_rsvp_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_event_rsvp_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();

    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    if auth.user.id != user_id {
        perms.needs(Permission::CalendarEventManage);
    } else {
        perms.needs(Permission::CalendarEventRsvp);
    }
    perms.check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    s.data()
        .calendar_event_rsvp_delete(req.event_id, user_id)
        .await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

    if auth.user.id != user_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarRsvpDelete {
            event_id: req.event_id,
            seq: None,
            user_id,
        })
        .await?;
    }

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarRsvpDelete {
            channel_id: req.channel_id,
            event_id: req.event_id,
            user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Calendar overwrite list
#[handler(routes::calendar_overwrite_list)]
async fn calendar_overwrite_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let overwrites = s.data().calendar_overwrite_list(req.event_id).await?;
    Ok(Json(overwrites))
}

/// Calendar overwrite get
#[handler(routes::calendar_overwrite_get)]
async fn calendar_overwrite_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let overwrite = s
        .data()
        .calendar_overwrite_get(req.event_id, req.seq)
        .await?;
    Ok(Json(overwrite))
}

/// Calendar overwrite update
#[handler(routes::calendar_overwrite_update)]
async fn calendar_overwrite_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::CalendarEventManage)
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = data.calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let old_overwrite = data
        .calendar_overwrite_get(req.event_id, req.seq)
        .await
        .ok();

    let overwrite = data
        .calendar_overwrite_put(req.event_id, req.seq, req.overwrite.clone())
        .await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

    let sync_event;
    if let Some(old) = old_overwrite {
        sync_event = MessageSync::CalendarOverwriteUpdate {
            channel_id: req.channel_id,
            overwrite: overwrite.clone(),
        };

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarOverwriteUpdate {
            event_id: req.event_id,
            seq: req.seq,
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
            channel_id: req.channel_id,
            overwrite: overwrite.clone(),
        };

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarOverwriteCreate {
            event_id: req.event_id,
            seq: req.seq,
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
#[handler(routes::calendar_overwrite_delete)]
async fn calendar_overwrite_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::CalendarEventManage)
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let overwrite = s
        .data()
        .calendar_overwrite_get(req.event_id, req.seq)
        .await?;
    s.data()
        .calendar_overwrite_delete(req.event_id, req.seq)
        .await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

    let al = auth.audit_log(room_id);
    al.commit_success(AuditLogEntryType::CalendarOverwriteDelete {
        event_id: req.event_id,
        seq: req.seq,
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
            channel_id: req.channel_id,
            event_id: req.event_id,
            seq: req.seq,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Calendar overwrite RSVP list
#[handler(routes::calendar_overwrite_rsvp_list)]
async fn calendar_overwrite_rsvp_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_rsvp_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let mut participants = s
        .data()
        .calendar_overwrite_rsvp_list(req.event_id, req.seq, req.query.clone())
        .await?;

    if req.query.include_member && !participants.is_empty() {
        let user_ids: Vec<UserId> = participants.iter().map(|p| p.user_id).collect();
        let users = srv.users.get_many(&user_ids).await?;
        let mut users_map: HashMap<_, _> = users.into_iter().map(|u| (u.id, u)).collect();

        let mut members_map = HashMap::new();
        if let Some(room_id) = chan.room_id {
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

/// Calendar overwrite RSVP put
#[handler(routes::calendar_overwrite_rsvp_put)]
async fn calendar_overwrite_rsvp_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_rsvp_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::ChannelView);
    perms.needs(Permission::CalendarEventRsvp);

    if auth.user.id != user_id {
        return Err(ApiError::from_code(ErrorCode::CannotRsvpOtherPeople).into());
    }

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

    match req.participant.status {
        CalendarRsvpStatus::Interested => {
            s.data()
                .calendar_overwrite_rsvp_put(req.event_id, req.seq, user_id, true)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarOverwriteRsvpCreate {
                    channel_id: req.channel_id,
                    event_id: req.event_id,
                    seq: req.seq,
                    participant: CalendarEventParticipant {
                        user_id,
                        status: req.participant.status,
                        user: None,
                        member: None,
                    },
                },
            )
            .await?;
        }
        CalendarRsvpStatus::Uninterested => {
            s.data()
                .calendar_overwrite_rsvp_put(req.event_id, req.seq, user_id, false)
                .await?;

            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::CalendarOverwriteRsvpDelete {
                    channel_id: req.channel_id,
                    event_id: req.event_id,
                    seq: req.seq,
                    user_id,
                },
            )
            .await?;
        }
    }

    Ok(StatusCode::OK)
}

/// Calendar overwrite RSVP delete
#[handler(routes::calendar_overwrite_rsvp_delete)]
async fn calendar_overwrite_rsvp_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::calendar_overwrite_rsvp_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();

    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::ChannelView);
    if auth.user.id != user_id {
        perms.needs(Permission::CalendarEventManage);
    } else {
        perms.needs(Permission::CalendarEventRsvp);
    }

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_calendar() {
        return Err(ApiError::from_code(ErrorCode::ChannelIsNotACalendar).into());
    }

    let event = s.data().calendar_event_get(req.event_id).await?;
    if event.channel_id != req.channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownCalendarEvent).into());
    }

    s.data()
        .calendar_overwrite_rsvp_delete(req.event_id, req.seq, user_id)
        .await?;

    let room_id = chan
        .room_id
        .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

    if auth.user.id != user_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::CalendarRsvpDelete {
            event_id: req.event_id,
            seq: Some(req.seq),
            user_id,
        })
        .await?;
    }

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::CalendarOverwriteRsvpDelete {
            channel_id: req.channel_id,
            event_id: req.event_id,
            seq: req.seq,
            user_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(calendar_event_list_user))
        .routes(routes2!(calendar_event_list))
        .routes(routes2!(calendar_event_create))
        .routes(routes2!(calendar_event_get))
        .routes(routes2!(calendar_event_update))
        .routes(routes2!(calendar_event_delete))
        .routes(routes2!(calendar_event_rsvp_list))
        .routes(routes2!(calendar_event_rsvp_get))
        .routes(routes2!(calendar_event_rsvp_put))
        .routes(routes2!(calendar_event_rsvp_delete))
        .routes(routes2!(calendar_overwrite_list))
        .routes(routes2!(calendar_overwrite_get))
        .routes(routes2!(calendar_overwrite_update))
        .routes(routes2!(calendar_overwrite_delete))
        .routes(routes2!(calendar_overwrite_rsvp_list))
        .routes(routes2!(calendar_overwrite_rsvp_put))
        .routes(routes2!(calendar_overwrite_rsvp_delete))
}
