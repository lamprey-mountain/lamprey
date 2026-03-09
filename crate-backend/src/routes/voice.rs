use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{
    misc::UserIdReq,
    util::{Changes, Time},
    voice::{
        CallCreate, CallDeleteParams, CallPatch, RingEligibility, RingStart, RingStop, SfuCommand,
        SfuPermissions, VoiceState, VoiceStateMove, VoiceStateMoveBulk, VoiceStatePatch,
    },
    AuditLogEntryType, ChannelId, ChannelType, MessageSync, PaginationResponse, Permission,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;

use crate::error::Result;
use crate::{Error, ServerState};

/// Voice state get
#[utoipa::path(
    get,
    path = "/voice/{channel_id}/member/{user_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.scope.full"],
    responses(
        (status = OK, body = VoiceState, description = "ok"),
    )
)]
async fn voice_state_get(
    Path((channel_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let state = srv.voice.state_get(target_user_id);
    if let Some(state) = state {
        Ok(Json(state))
    } else {
        Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)))
    }
}

/// Voice state patch
#[utoipa::path(
    patch,
    path = "/voice/{channel_id}/member/{user_id}",
    tags = ["voice", "badge.scope.full", "badge.perm-opt.VoiceMute", "badge.perm-opt.VoiceDeafen", "badge.perm-opt.VoiceRequest", "badge.perm-opt.VoiceMove", "badge.audit-log.MemberMove", "badge.audit-log.MemberUpdate"],
    params(
        ("channel_id" = ChannelId, Path, description = "Channel id"),
        ("user_id" = UserIdReq, Path, description = "User id"),
    ),
    request_body = VoiceStatePatch,
    responses(
        (status = OK, body = VoiceState, description = "ok"),
    )
)]
async fn voice_state_patch(
    Path((channel_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<VoiceStatePatch>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let Some(mut old_state) = srv.voice.state_get(target_user_id) else {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    };
    let chan = srv.channels.get(channel_id, None).await?;
    chan.ensure_has_voice()?;

    // handle move
    if let Some(new_channel_id) = json.channel_id {
        perms.ensure(Permission::VoiceMove)?;
        let target_chan = srv.channels.get(new_channel_id, None).await?;
        if target_chan.room_id != chan.room_id {
            return Err(ApiError::from_code(ErrorCode::CannotMoveToDifferentRoom).into());
        }
        let target_perms = srv
            .perms
            .for_channel(target_user_id, new_channel_id)
            .await?;
        target_perms.ensure(Permission::ViewChannel)?;

        let old_channel_id = old_state.channel_id;

        let _ = s.broadcast_sfu(SfuCommand::VoiceState {
            user_id: target_user_id,
            state: None,
            permissions: SfuPermissions {
                speak: target_perms.has(Permission::VoiceSpeak),
                video: target_perms.has(Permission::VoiceVideo),
                priority: target_perms.has(Permission::VoicePriority),
            },
        });

        old_state.channel_id = new_channel_id;
        srv.voice.state_put(old_state.clone()).await;

        if let Some(room_id) = chan.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::MemberMove {
                user_id: target_user_id,
                changes: Changes::new()
                    .change("thread_id", &old_channel_id, &channel_id)
                    .build(),
            })
            .await?;
        }
    }

    // handle mute/deaf/suppress
    // TODO: audit logging for suppress
    if json.mute.is_some() || json.deaf.is_some() || json.suppress.is_some() {
        if json.mute.is_some() {
            perms.ensure(Permission::VoiceMute)?;
        }
        if json.deaf.is_some() {
            perms.ensure(Permission::VoiceDeafen)?;
        }
        if json.suppress.is_some() {
            perms.ensure(Permission::VoiceMute)?;
        }

        let mute = json.mute.unwrap_or(old_state.mute);
        let deaf = json.deaf.unwrap_or(old_state.deaf);
        let suppress = json.suppress.unwrap_or(old_state.suppress);

        let state = VoiceState {
            mute,
            deaf,
            suppress,
            ..old_state
        };

        let perms_user = srv.perms.for_channel(target_user_id, channel_id).await?;
        let _ = s.broadcast_sfu(SfuCommand::VoiceState {
            user_id: target_user_id,
            state: Some(state.clone()),
            permissions: SfuPermissions {
                speak: perms_user.has(Permission::VoiceSpeak),
                video: perms_user.has(Permission::VoiceVideo),
                priority: perms_user.has(Permission::VoicePriority),
            },
        });

        srv.voice.state_put(state.clone()).await;
        old_state = state;

        if let Some(room_id) = chan.room_id {
            if json.mute.is_some() || json.deaf.is_some() {
                let changes = Changes::new()
                    .change("mute", &json.mute.unwrap_or(old_state.mute), &mute)
                    .change("deaf", &json.deaf.unwrap_or(old_state.deaf), &deaf)
                    .build();
                if !changes.is_empty() {
                    let al = auth.audit_log(room_id);
                    al.commit_success(AuditLogEntryType::MemberUpdate {
                        room_id,
                        user_id: target_user_id,
                        changes,
                    })
                    .await?;
                }
            }
        }
    }

    if let Some(requested_to_speak_at) = json.requested_to_speak_at {
        perms.ensure(Permission::VoiceRequest)?;
        if target_user_id != auth.user.id {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }

        let new_requested_to_speak_at = if requested_to_speak_at.is_some() {
            Some(Time::now_utc())
        } else {
            None
        };

        let state = VoiceState {
            requested_to_speak_at: new_requested_to_speak_at,
            ..old_state
        };

        let perms_user = srv.perms.for_channel(target_user_id, channel_id).await?;
        let _ = s.broadcast_sfu(SfuCommand::VoiceState {
            user_id: target_user_id,
            state: Some(state.clone()),
            permissions: SfuPermissions {
                speak: perms_user.has(Permission::VoiceSpeak),
                video: perms_user.has(Permission::VoiceVideo),
                priority: perms_user.has(Permission::VoicePriority),
            },
        });

        srv.voice.state_put(state.clone()).await;
        old_state = state;
    }

    if let Some(room_id) = chan.room_id {
        let d = s.data();
        let res = d.room_member_get(room_id, target_user_id).await?;
        let user = srv.users.get(target_user_id, None).await?;
        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::RoomMemberUpdate {
                member: res.clone(),
                user,
            },
        )
        .await?;
    }

    Ok(Json(old_state))
}

/// Voice state disconnect
#[utoipa::path(
    delete,
    path = "/voice/{channel_id}/member/{user_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.scope.full", "badge.perm.VoiceDisconnect", "badge.audit-log.MemberDisconnect"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_state_disconnect(
    Path((channel_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
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
    let chan = srv.channels.get(channel_id, None).await?;
    chan.ensure_has_voice()?;
    let _ = s.broadcast_sfu(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: None,
        permissions: SfuPermissions {
            speak: target_perms.has(Permission::VoiceSpeak),
            video: target_perms.has(Permission::VoiceVideo),
            priority: target_perms.has(Permission::VoicePriority),
        },
    });
    if let Some(room_id) = chan.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MemberDisconnect {
            channel_id,
            user_id: target_user_id,
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Voice state disconnect all
#[utoipa::path(
    delete,
    path = "/voice/{channel_id}/member",
    params(
        ("channel_id", description = "Channel id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.scope.full", "badge.perm.VoiceDisconnect", "badge.audit-log.MemberDisconnectAll"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_state_disconnect_all(
    Path((channel_id,)): Path<(ChannelId,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::VoiceDisconnect)?;
    let thread = srv.channels.get(channel_id, None).await?;
    thread.ensure_has_voice()?;
    srv.voice.disconnect_everyone(channel_id).await?;
    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MemberDisconnectAll { channel_id })
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Voice state move
#[utoipa::path(
    post,
    path = "/voice/{channel_id}/member/{user_id}/move",
    tags = ["voice", "badge.scope.full", "badge.perm.VoiceMove", "badge.audit-log.MemberMove"],
    params(
        ("channel_id" = ChannelId, Path, description = "Channel id"),
        ("user_id" = UserIdReq, Path, description = "User id"),
    ),
    request_body = VoiceStateMove,
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_state_move(
    Path((channel_id, target_user_id)): Path<(ChannelId, UserIdReq)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<VoiceStateMove>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms_source = srv.perms.for_channel(auth.user.id, channel_id).await?;
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
        return Err(ApiError::from_code(ErrorCode::NotConnectedToAnyThread).into());
    };

    let state = VoiceState {
        channel_id: json.target_id,
        ..old
    };

    let target_perms = srv.perms.for_channel(target_user_id, channel_id).await?;
    let _ = s.broadcast_sfu(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: None,
        permissions: SfuPermissions {
            speak: target_perms.has(Permission::VoiceSpeak),
            video: target_perms.has(Permission::VoiceVideo),
            priority: target_perms.has(Permission::VoicePriority),
        },
    });

    let chan = srv.channels.get(channel_id, None).await?;
    chan.ensure_has_voice()?;
    if let Some(room_id) = chan.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MemberMove {
            user_id: target_user_id,
            changes: Changes::new()
                .change("thread_id", &old.channel_id, &state.channel_id)
                .build(),
        })
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Voice state move bulk (TODO)
// TODO: rename this to "voice state move" and deprecate current voice state move route?
#[utoipa::path(
    post,
    path = "/voice/{channel_id}/move",
    params(("channel_id", description = "Channel id")),
    tags = ["voice", "badge.scope.full", "badge.perm.VoiceMove"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_state_move_bulk(
    Path((channel_id,)): Path<(ChannelId,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(_json): Json<VoiceStateMoveBulk>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms_source = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms_source.ensure(Permission::ViewChannel)?;
    perms_source.ensure(Permission::VoiceMove)?;
    let chan = srv.channels.get(channel_id, None).await?;
    chan.ensure_has_voice()?;

    Ok(Error::Unimplemented)
}

/// Voice state list
#[utoipa::path(
    get,
    path = "/voice/{channel_id}/member",
    params(
        ("channel_id", description = "Channel id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice", "badge.scope.full"],
    responses((status = OK, description = "ok"))
)]
async fn voice_state_list(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let chan = srv.channels.get(channel_id, None).await?;
    chan.ensure_has_voice()?;
    let states: Vec<_> = srv
        .voice
        .state_list()
        .into_iter()
        .filter(|s| s.channel_id == channel_id)
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

/// Voice call create
#[utoipa::path(
    post,
    path = "/voice",
    tags = ["voice", "badge.scope.full"],
    request_body = CallCreate,
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_call_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CallCreate>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, json.channel_id)
        .await?;
    perms.ensure_all(&[Permission::ViewChannel, Permission::CallUpdate])?;
    let channel = s.services().channels.get(json.channel_id, None).await?;
    if channel.ty != ChannelType::Broadcast {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }
    s.services().voice.call_create(json).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Voice call delete
#[utoipa::path(
    delete,
    path = "/voice/{channel_id}",
    params(("channel_id", description = "Channel id")),
    tags = ["voice", "badge.scope.full"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_call_delete(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Query(params): Query<CallDeleteParams>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, channel_id)
        .await?;
    perms.ensure_all(&[Permission::ViewChannel, Permission::CallUpdate])?;
    let channel = s.services().channels.get(channel_id, None).await?;
    if channel.ty != ChannelType::Broadcast {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }
    if params.force {
        perms.ensure(Permission::VoiceDisconnect)?;
    }
    s.services()
        .voice
        .call_delete(channel_id, params.force)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Voice call get
#[utoipa::path(
    get,
    path = "/voice/{channel_id}",
    params(("channel_id", description = "Channel id")),
    tags = ["voice", "badge.scope.full"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_call_get(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let call = s.services().voice.call_get(channel_id)?;
    Ok(Json(call))
}

/// Voice call update
// TODO: return the updated call object
#[utoipa::path(
    patch,
    path = "/voice/{channel_id}",
    tags = ["voice", "badge.scope.full"],
    params(
        ("channel_id" = ChannelId, Path, description = "Channel id"),
    ),
    request_body = CallPatch,
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_call_update(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<CallPatch>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, channel_id)
        .await?;
    perms.ensure_all(&[Permission::ViewChannel, Permission::CallUpdate])?;
    s.services().voice.call_update(channel_id, json)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Voice ring check
#[utoipa::path(
    get,
    path = "/voice/{channel_id}/ring",
    params(("channel_id", description = "Channel id")),
    tags = ["voice", "badge.scope.full"],
    responses(
        (status = OK, body = RingEligibility, description = "ok"),
    )
)]
async fn voice_ring_check(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;
    let channel = s
        .services()
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?;
    let ringable = matches!(channel.ty, ChannelType::Dm | ChannelType::Gdm);
    Ok(Json(RingEligibility { ringable }))
}

/// Voice ring start (TODO)
#[utoipa::path(
    post,
    path = "/voice/{channel_id}/ring",
    params(("channel_id", description = "Channel id")),
    tags = ["voice", "badge.scope.full"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_ring_start(
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<RingStart>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Voice ring stop (TODO)
#[utoipa::path(
    post,
    path = "/voice/{channel_id}/ring/ack",
    params(("channel_id", description = "Channel id")),
    tags = ["voice", "badge.scope.full"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_ring_stop(
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<RingStop>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
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
        .routes(routes!(voice_call_create))
        .routes(routes!(voice_call_delete))
        .routes(routes!(voice_call_get))
        .routes(routes!(voice_call_update))
        .routes(routes!(voice_ring_check))
        .routes(routes!(voice_ring_start))
        .routes(routes!(voice_ring_stop))
}
