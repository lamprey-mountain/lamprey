use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    misc::UserIdReq,
    util::Changes,
    voice::{SfuCommand, VoiceState},
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, PaginationResponse, Permission, ThreadId,
};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};

use crate::error::Result;
use crate::{Error, ServerState};

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
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_state_get(
    Path((thread_id, target_user_id)): Path<(ThreadId, UserIdReq)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    let state = srv.users.voice_state_get(thread_id, target_user_id);
    Ok(Json(state))
}

/// Voice state disconnect (TODO)
#[utoipa::path(
    delete,
    path = "/voice/{thread_id}/member/{user_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_state_disconnect(
    Path((thread_id, target_user_id)): Path<(ThreadId, UserIdReq)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::VoiceDisconnect)?;
    let Some(_state) = srv.users.voice_state_get(thread_id, target_user_id) else {
        return Ok(());
    };
    let _ = s.sushi_sfu.send(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: None,
    });
    srv.users.voice_state_remove(&target_user_id);
    let thread = srv.threads.get(thread_id, None).await?;
    if let Some(room_id) = thread.room_id {
        let data = s.data();
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::MemberDisconnect {
                thread_id,
                user_id: target_user_id,
            },
        })
        .await?;
    }
    Ok(())
}

/// Voice state move
#[utoipa::path(
    post,
    path = "/voice/{thread_id}/member/{user_id}/move",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_state_move(
    Path((thread_id, target_user_id)): Path<(ThreadId, UserIdReq)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<VoiceStateMove>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms_source = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms_source.ensure_view()?;
    perms_source.ensure(Permission::VoiceMove)?;
    let perms_target = srv.perms.for_thread(auth_user.id, json.target_id).await?;
    perms_target.ensure_view()?;
    perms_target.ensure(Permission::VoiceMove)?;
    let perms_user = srv.perms.for_thread(target_user_id, json.target_id).await?;
    perms_user.ensure_view()?;

    let Some(old) = srv.users.voice_state_get(thread_id, target_user_id) else {
        return Err(Error::BadStatic("not connected to any thread"));
    };

    let state = VoiceState {
        thread_id: json.target_id,
        ..old
    };
    let _ = s.sushi_sfu.send(SfuCommand::VoiceState {
        user_id: target_user_id,
        state: Some(state.clone()),
    });

    let thread = srv.threads.get(thread_id, None).await?;
    if let Some(room_id) = thread.room_id {
        let data = s.data();
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::MemberMove {
                user_id: target_user_id,
                changes: Changes::new()
                    .change("thread_id", &old.thread_id, &state.thread_id)
                    .build(),
            },
        })
        .await?;
    }

    Ok(())
}

#[derive(Debug, ToSchema, Deserialize)]
struct VoiceStateMove {
    target_id: ThreadId,
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
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    let states: Vec<_> = srv
        .users
        .voice_states_list()
        .into_iter()
        .filter(|s| s.thread_id == thread_id)
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

/// Voice region list
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
        .routes(routes!(voice_state_disconnect))
        .routes(routes!(voice_state_move))
        .routes(routes!(voice_state_list))
        .routes(routes!(voice_region_list))
}
