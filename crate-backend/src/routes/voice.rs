use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::misc::UserIdReq;
use common::v1::types::util::{Changes, Time};
use common::v1::types::voice::{RingEligibility, SfuCommand, SfuPermissions, VoiceState};
use common::v1::types::{
    AuditLogEntryType, ChannelType, MessageSync, PaginationResponse, Permission,
};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use super::util::Auth;

use crate::error::Result;
use crate::{routes2, Error, ServerState};

/// Voice state get
#[handler(routes::voice_state_get)]
async fn voice_state_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let state = srv.voice.state_get(target_user_id);
    if let Some(state) = state {
        Ok(Json(state))
    } else {
        Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)))
    }
}

/// Voice state patch
#[handler(routes::voice_state_patch)]
async fn voice_state_patch(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_patch::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let Some(mut old_state) = srv.voice.state_get(target_user_id) else {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    };
    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ensure_has_voice()?;

    // handle move
    if let Some(new_channel_id) = req.state.channel_id {
        perms.ensure(Permission::VoiceMove)?;
        let target_chan = srv.channels.get(new_channel_id, None).await?;
        if target_chan.room_id != chan.room_id {
            return Err(ApiError::from_code(ErrorCode::CannotMoveToDifferentRoom).into());
        }
        let target_perms = srv
            .perms
            .for_channel(target_user_id, new_channel_id)
            .await?;
        target_perms.ensure(Permission::ChannelView)?;

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
                    .change("thread_id", &old_channel_id, &req.channel_id)
                    .build(),
            })
            .await?;
        }
    }

    // handle mute/deaf/suppress
    if req.state.mute.is_some() || req.state.deaf.is_some() || req.state.suppress.is_some() {
        if req.state.mute.is_some() {
            perms.ensure(Permission::VoiceMute)?;
        }
        if req.state.deaf.is_some() {
            perms.ensure(Permission::VoiceDeafen)?;
        }
        if req.state.suppress.is_some() {
            perms.ensure(Permission::VoiceMute)?;
        }

        let mute = req.state.mute.unwrap_or(old_state.mute);
        let deaf = req.state.deaf.unwrap_or(old_state.deaf);
        let suppress = req.state.suppress.unwrap_or(old_state.suppress);

        let state = VoiceState {
            mute,
            deaf,
            suppress,
            ..old_state
        };

        let perms_user = srv
            .perms
            .for_channel(target_user_id, req.channel_id)
            .await?;
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
            let changes = Changes::new()
                .change("mute", &req.state.mute.unwrap_or(old_state.mute), &mute)
                .change("deaf", &req.state.deaf.unwrap_or(old_state.deaf), &deaf)
                .change(
                    "suppress",
                    &req.state.suppress.unwrap_or(old_state.suppress),
                    &suppress,
                )
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

    if let Some(requested_to_speak_at) = req.state.requested_to_speak_at {
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

        let perms_user = srv
            .perms
            .for_channel(target_user_id, req.channel_id)
            .await?;
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
#[handler(routes::voice_state_disconnect)]
async fn voice_state_disconnect(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_disconnect::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    perms.ensure(Permission::VoiceMove)?;
    let target_perms = srv
        .perms
        .for_channel(target_user_id, req.channel_id)
        .await?;
    let Some(_state) = srv.voice.state_get(target_user_id) else {
        return Ok(StatusCode::NO_CONTENT);
    };
    let chan = srv.channels.get(req.channel_id, None).await?;
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
            channel_id: req.channel_id,
            user_id: target_user_id,
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Voice state disconnect all
#[handler(routes::voice_state_disconnect_all)]
async fn voice_state_disconnect_all(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_disconnect_all::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    perms.ensure(Permission::VoiceMove)?;
    let thread = srv.channels.get(req.channel_id, None).await?;
    thread.ensure_has_voice()?;
    srv.voice.disconnect_everyone(req.channel_id).await?;
    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::MemberDisconnectAll {
            channel_id: req.channel_id,
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Voice state move
#[handler(routes::voice_state_move)]
async fn voice_state_move(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_move::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms_source = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms_source.ensure(Permission::ChannelView)?;
    perms_source.ensure(Permission::VoiceMove)?;
    let perms_target = srv
        .perms
        .for_channel(auth.user.id, req.move_req.target_id)
        .await?;
    perms_target.ensure(Permission::ChannelView)?;
    perms_target.ensure(Permission::VoiceMove)?;
    let _perms_user = srv
        .perms
        .for_channel(target_user_id, req.move_req.target_id)
        .await?;
    perms_target.ensure(Permission::ChannelView)?;

    let Some(old) = srv.voice.state_get(target_user_id) else {
        return Err(ApiError::from_code(ErrorCode::NotConnectedToAnyThread).into());
    };

    let state = VoiceState {
        channel_id: req.move_req.target_id,
        ..old
    };

    let target_perms = srv
        .perms
        .for_channel(target_user_id, req.channel_id)
        .await?;
    let _ = s.broadcast_sfu(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: None,
        permissions: SfuPermissions {
            speak: target_perms.has(Permission::VoiceSpeak),
            video: target_perms.has(Permission::VoiceVideo),
            priority: target_perms.has(Permission::VoicePriority),
        },
    });

    let chan = srv.channels.get(req.channel_id, None).await?;
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
#[handler(routes::voice_state_move_bulk)]
async fn voice_state_move_bulk(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_move_bulk::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms_source = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms_source.ensure(Permission::ChannelView)?;
    perms_source.ensure(Permission::VoiceMove)?;
    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ensure_has_voice()?;

    Ok(Error::Unimplemented)
}

/// Voice state list
#[handler(routes::voice_state_list)]
async fn voice_state_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ensure_has_voice()?;
    let states: Vec<_> = srv
        .voice
        .state_list()
        .into_iter()
        .filter(|s| s.channel_id == req.channel_id)
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
#[handler(routes::voice_call_create)]
async fn voice_call_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_call_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.channel_id)
        .await?;
    perms.ensure_all(&[Permission::ChannelView, Permission::CallUpdate])?;
    let channel = s.services().channels.get(req.channel_id, None).await?;
    if channel.ty != ChannelType::Broadcast {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }
    s.services().voice.call_create(req.call).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Voice call delete
#[handler(routes::voice_call_delete)]
async fn voice_call_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_call_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.channel_id)
        .await?;
    perms.ensure_all(&[Permission::ChannelView, Permission::CallUpdate])?;
    let channel = s.services().channels.get(req.channel_id, None).await?;
    if channel.ty != ChannelType::Broadcast {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }
    if req.params.force {
        perms.ensure(Permission::VoiceMove)?;
    }
    s.services()
        .voice
        .call_delete(req.channel_id, req.params.force)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Voice call get
#[handler(routes::voice_call_get)]
async fn voice_call_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_call_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.channel_id)
        .await?;
    perms.ensure(Permission::ChannelView)?;
    let call = s.services().voice.call_get(req.channel_id)?;
    Ok(Json(call))
}

/// Voice call update
// TODO: return the updated call object
#[handler(routes::voice_call_patch)]
async fn voice_call_patch(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_call_patch::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.channel_id)
        .await?;
    perms.ensure_all(&[Permission::ChannelView, Permission::CallUpdate])?;
    s.services().voice.call_update(req.channel_id, req.call)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Voice ring check
#[handler(routes::voice_ring_eligibility)]
async fn voice_ring_eligibility(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_ring_eligibility::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.channel_id)
        .await?;
    perms.ensure(Permission::ChannelView)?;
    let channel = s
        .services()
        .channels
        .get(req.channel_id, Some(auth.user.id))
        .await?;
    let ringable = matches!(channel.ty, ChannelType::Dm | ChannelType::Gdm);
    Ok(Json(RingEligibility { ringable }))
}

/// Voice ring start (TODO)
#[handler(routes::voice_ring_start)]
async fn voice_ring_start(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::voice_ring_start::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Voice ring stop (TODO)
#[handler(routes::voice_ring_stop)]
async fn voice_ring_stop(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::voice_ring_stop::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(voice_state_get))
        .routes(routes2!(voice_state_patch))
        .routes(routes2!(voice_state_disconnect))
        .routes(routes2!(voice_state_disconnect_all))
        .routes(routes2!(voice_state_move))
        .routes(routes2!(voice_state_move_bulk))
        .routes(routes2!(voice_state_list))
        .routes(routes2!(voice_call_create))
        .routes(routes2!(voice_call_delete))
        .routes(routes2!(voice_call_get))
        .routes(routes2!(voice_call_patch))
        .routes(routes2!(voice_ring_eligibility))
        .routes(routes2!(voice_ring_start))
        .routes(routes2!(voice_ring_stop))
}
