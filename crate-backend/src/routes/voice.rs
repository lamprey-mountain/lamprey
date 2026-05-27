use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Time};
use common::v1::types::voice::{RingEligibility, VoiceStateUpdate};
use common::v1::types::{
    AuditLogEntryType, ChannelType, MessageSync, PaginationResponse, Permission,
};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use super::util::Auth;

use crate::error::{Error, Result};
use crate::{routes2, ServerState};

/// Voice state get
#[handler(routes::voice_state_get)]
async fn voice_state_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let target_user_id = req.user_id.local_unwrap_or(auth.user.id)?;
    let handle = srv
        .voice
        .state_get(req.channel_id, target_user_id)
        .ok_or_else(|| Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)))?;
    let state = handle.inner().clone();

    if state.channel_id != req.channel_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    Ok(Json(state))
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

    let srv = s.services();
    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;

    let handle = srv
        .voice
        .state_get(req.channel_id, req.user_id.unwrap_or(auth.user.id))
        .ok_or_else(|| Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)))?;
    let old_state = handle.inner().clone();
    let target_user_id = old_state.user_id;

    if old_state.channel_id != req.channel_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ensure_has_voice()?;

    let mut state_changed = false;
    let mut current_state = old_state.clone();

    // handle move
    if let Some(new_channel_id) = req.state.channel_id {
        perms.needs(Permission::VoiceMove);
        let target_chan = srv.channels.get(new_channel_id, None).await?;
        if target_chan.room_id != chan.room_id {
            return Err(ApiError::from_code(ErrorCode::CannotMoveToDifferentRoom).into());
        }

        let _target_perms = srv
            .perms
            .for_channel3(Some(target_user_id), new_channel_id)
            .await?
            .ensure_view()?;

        let old_channel_id = current_state.channel_id;
        current_state.channel_id = new_channel_id;
        state_changed = true;

        if let Some(room_id) = chan.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::MemberMove {
                user_id: target_user_id,
                changes: Changes::new()
                    .change("thread_id", &old_channel_id, &new_channel_id)
                    .build(),
            })
            .await?;
        }
    }

    // handle moderator mute/deaf/suppress
    if req.state.mute.is_some() || req.state.deaf.is_some() || req.state.suppress.is_some() {
        perms.needs(Permission::VoiceMute);
        perms.needs(Permission::VoiceDeafen);

        let mute = req.state.mute.unwrap_or(current_state.mute);
        let deaf = req.state.deaf.unwrap_or(current_state.deaf);
        let suppress = req.state.suppress.unwrap_or(current_state.suppress);

        current_state.mute = mute;
        current_state.deaf = deaf;
        current_state.suppress = suppress;
        state_changed = true;

        if let Some(room_id) = chan.room_id {
            let changes = Changes::new()
                .change("mute", &old_state.mute, &mute)
                .change("deaf", &old_state.deaf, &deaf)
                .change("suppress", &old_state.suppress, &suppress)
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
        perms.needs(Permission::VoiceRequest);
        if target_user_id != auth.user.id {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }

        let new_requested_to_speak_at = if requested_to_speak_at.is_some() {
            Some(Time::now_utc())
        } else {
            None
        };

        current_state.requested_to_speak_at = new_requested_to_speak_at;
        state_changed = true;
    }

    perms.check()?;

    if state_changed {
        // FIXME: handle updating `suppress` field
        srv.voice.state_update(
            target_user_id,
            VoiceStateUpdate {
                channel_id: current_state.channel_id,
                self_deaf: current_state.self_deaf,
                self_mute: current_state.self_mute,
                self_video: current_state.self_video,
                screenshare: Some(current_state.screenshare.clone()),
            },
        )?;
    }

    if let Some(room_id) = chan.room_id {
        let mut d = s.data();
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

    Ok(Json(current_state))
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

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ensure_has_voice()?;

    let target_user_id = req.user_id.local_unwrap_or(auth.user.id)?;

    // If there's a single voice state for this user in this channel, use it
    // to check self-disconnect logic. Otherwise, VoiceMove is required.
    if let Some(handle) = srv.voice.state_get(req.channel_id, target_user_id) {
        let state = handle.inner();
        if state.channel_id == req.channel_id {
            let mut perms = srv
                .perms
                .for_channel3(Some(auth.user.id), req.channel_id)
                .await?
                .ensure_view()?;

            if state.user_id != auth.user.id {
                perms.needs(Permission::VoiceMove);
            }
            perms.check()?;

            srv.voice.state_destroy(req.channel_id, state.user_id)?;

            if let Some(room_id) = chan.room_id {
                let al = auth.audit_log(room_id);
                al.commit_success(AuditLogEntryType::MemberDisconnect {
                    channel_id: req.channel_id,
                    user_id: state.user_id,
                })
                .await?;
            }
            return Ok(StatusCode::NO_CONTENT);
        }
    }

    // user has multiple voice states or none in this channel; disconnect all
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::VoiceMove)
        .check()?;

    let body_count = srv
        .voice
        .call_disconnect_all_user(req.channel_id, target_user_id)
        .await?;

    if body_count > 0 {
        if let Some(room_id) = chan.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::MemberDisconnect {
                channel_id: req.channel_id,
                user_id: target_user_id,
            })
            .await?;
        }
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
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::VoiceMove)
        .check()?;

    let thread = srv.channels.get(req.channel_id, None).await?;
    thread.ensure_has_voice()?;

    srv.voice.call_disconnect_all(req.channel_id).await?;

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

    // let target_user_id = req.user_id.unwrap_or(auth.user.id);
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::VoiceMove)
        .check()?;
    srv.perms
        .for_channel3(Some(auth.user.id), req.move_req.target_id)
        .await?
        .ensure_view()?
        .needs(Permission::VoiceMove)
        .check()?;

    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ensure_has_voice()?;

    // let Some(old) = srv.voice.state_get(target_user_id) else {
    //     return Err(ApiError::from_code(ErrorCode::NotConnectedToAnyThread).into());
    // };

    // let state = VoiceState {
    //     channel_id: req.move_req.target_id,
    //     ..old
    // };

    // let target_perms = srv
    //     .perms
    //     .for_channel3(Some(target_user_id), req.channel_id)
    //     .await?;
    // let _ = s.broadcast_sfu(SfuCommand::VoiceState {
    //     user_id: target_user_id,
    //     state: None,
    //     permissions: SfuPermissions::from_bools(
    //         target_perms.has(Permission::VoiceSpeak),
    //         target_perms.has(Permission::VoiceVideo),
    //         target_perms.has(Permission::VoicePriority),
    //     ),
    // });

    // let chan = srv.channels.get(req.channel_id, None).await?;
    // chan.ensure_has_voice()?;
    // if let Some(room_id) = chan.room_id {
    //     let al = auth.audit_log(room_id);
    //     al.commit_success(AuditLogEntryType::MemberMove {
    //         user_id: target_user_id,
    //         changes: Changes::new()
    //             .change("thread_id", &old.channel_id, &state.channel_id)
    //             .build(),
    //     })
    //     .await?;
    // }

    // Ok(StatusCode::NO_CONTENT)
    Ok(Error::Unimplemented)
}

/// Voice state move bulk
#[handler(routes::voice_state_move_bulk)]
async fn voice_state_move_bulk(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_state_move_bulk::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::VoiceMove)
        .check()?;
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
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let states: Vec<_> = srv
        .voice
        .state_list_by_channel(req.channel_id)
        .into_iter()
        .map(|h| h.inner().clone())
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

    s.services()
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::CallUpdate)
        .check()?;

    let channel = s.services().channels.get(req.channel_id, None).await?;
    if channel.ty != ChannelType::Broadcast {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }

    let call_handle = s
        .services()
        .voice
        .call_create(req.channel_id, req.call)
        .await?;
    Ok((StatusCode::CREATED, Json(call_handle.call().clone())))
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

    let mut perms = s
        .services()
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::CallUpdate);

    let channel = s.services().channels.get(req.channel_id, None).await?;
    if channel.ty != ChannelType::Broadcast {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }
    if req.params.force {
        perms.needs(Permission::VoiceMove);
    }
    perms.check()?;

    s.services()
        .voice
        .call_delete(req.channel_id, req.params.force);
    Ok(StatusCode::NO_CONTENT)
}

/// Voice call patch
#[handler(routes::voice_call_patch)]
async fn voice_call_patch(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_call_patch::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    s.services()
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::CallUpdate)
        .check()?;

    let call_handle = s.services().voice.call_update(req.channel_id, req.call)?;
    Ok((StatusCode::OK, Json(call_handle.call().clone())))
}

/// Voice call get
#[handler(routes::voice_call_get)]
async fn voice_call_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_call_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.services()
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let call = s
        .services()
        .voice
        .call_get(req.channel_id)
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownVoiceChannel,
        )))?;
    Ok(Json(call.call().clone()))
}

/// Voice ring eligibility
#[handler(routes::voice_ring_eligibility)]
async fn voice_ring_eligibility(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::voice_ring_eligibility::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let chan = s
        .services()
        .channels
        .get(req.channel_id, Some(auth.user.id))
        .await?;
    Ok(Json(RingEligibility {
        ringable: matches!(chan.ty, ChannelType::Dm | ChannelType::Gdm),
    }))
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
        .routes(routes2!(voice_state_move))
        .routes(routes2!(voice_state_move_bulk))
        .routes(routes2!(voice_state_disconnect))
        .routes(routes2!(voice_state_disconnect_all))
        .routes(routes2!(voice_state_list))
        .routes(routes2!(voice_call_create))
        .routes(routes2!(voice_call_delete))
        .routes(routes2!(voice_call_patch))
        .routes(routes2!(voice_call_get))
        .routes(routes2!(voice_ring_start))
        .routes(routes2!(voice_ring_stop))
        .routes(routes2!(voice_ring_eligibility))
}
