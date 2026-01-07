use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    misc::UserIdReq,
    util::Changes,
    voice::{
        SfuCommand, SfuPermissions, VoiceState, VoiceStateMove, VoiceStateMoveBulk, VoiceStatePatch,
    },
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelId, PaginationResponse, Permission,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth2, HeaderReason};

use crate::error::Result;
use crate::{Error, ServerState};

// TODO: rename thread_id to channel_id in all routes

/// Voice state get
#[utoipa::path(
    get,
    path = "/voice/{thread_id}/member/{user_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = OK, body = VoiceState, description = "ok"),
    )
)]
async fn voice_state_get(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let state = srv.voice.state_get(target_user_id);
    if let Some(state) = state {
        Ok(Json(state))
    } else {
        Err(Error::NotFound)
    }
}

/// Voice state patch (TODO)
#[utoipa::path(
    patch,
    path = "/voice/{thread_id}/member/{user_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = OK, body = VoiceState, description = "ok"),
    )
)]
async fn voice_state_patch(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Json(_json): Json<VoiceStatePatch>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let _state = srv.voice.state_get(target_user_id);
    Ok(Error::Unimplemented)
}

/// Voice state disconnect
#[utoipa::path(
    delete,
    path = "/voice/{thread_id}/member/{user_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.perm.VoiceDisconnect"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_state_disconnect(
    Path((channel_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::VoiceDisconnect)?;
    let target_perms = srv.perms.for_channel(target_user_id, channel_id).await?;
    let Some(_state) = srv.voice.state_get(target_user_id) else {
        return Ok(StatusCode::NO_CONTENT);
    };
    let _ = s.sushi_sfu.send(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: None,
        permissions: SfuPermissions {
            speak: target_perms.has(Permission::VoiceSpeak),
            video: target_perms.has(Permission::VoiceVideo),
            priority: target_perms.has(Permission::VoicePriority),
        },
    });
    let thread = srv.channels.get(channel_id, None).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::MemberDisconnect {
                channel_id,
                user_id: target_user_id,
            },
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Voice state disconnect all
#[utoipa::path(
    delete,
    path = "/voice/{thread_id}/member",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.perm.VoiceDisconnect"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_state_disconnect_all(
    Path((channel_id,)): Path<(ChannelId,)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::VoiceDisconnect)?;
    srv.voice.disconnect_everyone(channel_id)?;
    let thread = srv.channels.get(channel_id, None).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::MemberDisconnectAll { channel_id },
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Voice state move
#[utoipa::path(
    post,
    path = "/voice/{thread_id}/member/{user_id}/move",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.perm.VoiceMove"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_state_move(
    Path((thread_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<VoiceStateMove>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms_source = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms_source.ensure(Permission::ViewChannel)?;
    perms_source.ensure(Permission::VoiceMove)?;
    let perms_target = srv.perms.for_channel(auth.user.id, json.target_id).await?;
    perms_target.ensure(Permission::ViewChannel)?;
    perms_target.ensure(Permission::VoiceMove)?;
    let _perms_user = srv
        .perms
        .for_channel(target_user_id, json.target_id)
        .await?;
    perms_target.ensure(Permission::ViewChannel)?;

    let Some(old) = srv.voice.state_get(target_user_id) else {
        return Err(Error::BadStatic("not connected to any thread"));
    };

    let state = VoiceState {
        channel_id: json.target_id,
        ..old
    };

    let target_perms = srv.perms.for_channel(target_user_id, thread_id).await?;
    let _ = s.sushi_sfu.send(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: None,
        permissions: SfuPermissions {
            speak: target_perms.has(Permission::VoiceSpeak),
            video: target_perms.has(Permission::VoiceVideo),
            priority: target_perms.has(Permission::VoicePriority),
        },
    });

    let thread = srv.channels.get(thread_id, None).await?;
    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::MemberMove {
                user_id: target_user_id,
                changes: Changes::new()
                    .change("thread_id", &old.channel_id, &state.channel_id)
                    .build(),
            },
        })
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Voice state move bulk (TODO)
// TODO: rename this to "voice state move" and deprecate current voice state move route?
#[utoipa::path(
    post,
    path = "/voice/{thread_id}/move",
    params(("thread_id", description = "Thread id")),
    tags = ["voice", "badge.perm.VoiceMove"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_state_move_bulk(
    Path((thread_id,)): Path<(ChannelId,)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(_json): Json<VoiceStateMoveBulk>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms_source = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms_source.ensure(Permission::ViewChannel)?;
    perms_source.ensure(Permission::VoiceMove)?;

    Ok(Error::Unimplemented)
}

/// Voice state list
#[utoipa::path(
    get,
    path = "/voice/{thread_id}/member",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses((status = OK, description = "ok"))
)]
async fn voice_state_list(
    Path(thread_id): Path<ChannelId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, thread_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let states: Vec<_> = srv
        .voice
        .state_list()
        .into_iter()
        .filter(|s| s.channel_id == thread_id)
        .collect();
    // this endpoint doesn't support pagination, but the results are returned in
    // a PaginationResponse anyways for consistency
    let total = states.len() as u64;
    Ok(Json(PaginationResponse {
        items: states,
        total,
        has_more: false,
        cursor: None,
    }))
}

/// Voice region list (TODO)
#[utoipa::path(
    get,
    path = "/voice/region",
    tags = ["voice"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_region_list(State(_s): State<Arc<ServerState>>) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(voice_state_get))
        .routes(routes!(voice_state_patch))
        .routes(routes!(voice_state_disconnect))
        .routes(routes!(voice_state_disconnect_all))
        .routes(routes!(voice_state_move))
        .routes(routes!(voice_state_move_bulk))
        .routes(routes!(voice_state_list))
        .routes(routes!(voice_region_list))
}
