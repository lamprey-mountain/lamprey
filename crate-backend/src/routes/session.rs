use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, PaginationQuery,
    PaginationResponse, Session, SessionCreate, SessionId, SessionPatch, SessionStatus,
    SessionToken, SessionType, SessionWithToken,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::routes::util::{Auth, HeaderReason};
use crate::types::{DbSessionCreate, SessionIdReq};
use crate::ServerState;

use super::util::AuthRelaxed;
use crate::error::{Error, Result};

/// Session create
#[utoipa::path(
    post,
    path = "/session",
    tags = ["session"],
    responses(
        (status = CREATED, body = SessionWithToken, description = "success"),
    )
)]
pub async fn session_create(
    State(s): State<Arc<ServerState>>,
    Json(json): Json<SessionCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let token = SessionToken(Uuid::new_v4().to_string()); // TODO: is this secure enough
    let session = data
        .session_create(DbSessionCreate {
            token: token.clone(),
            name: json.name,
            expires_at: None,
            ty: SessionType::User,
            application_id: None,
        })
        .await?;
    let session_with_token = SessionWithToken { session, token };
    Ok((StatusCode::CREATED, Json(session_with_token)))
}

/// Session list
#[utoipa::path(
    get,
    path = "/session",
    tags = ["session"],
    params(PaginationQuery<SessionId>),
    responses(
        (status = OK, description = "List session success", body = PaginationResponse<Session>),
    )
)]
pub async fn session_list(
    Query(q): Query<PaginationQuery<SessionId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.session_list(auth_user.id, q).await?;
    Ok(Json(res))
}

/// Session update
#[utoipa::path(
    patch,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, body = Session, description = "success"),
        (status = NOT_MODIFIED, body = Session, description = "not modified"),
    )
)]
pub async fn session_update(
    Path(target_session_id): Path<SessionIdReq>,
    AuthRelaxed(auth_session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<SessionPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let target_session_id = match target_session_id {
        SessionIdReq::SessionSelf => auth_session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    let data = s.data();
    let srv = s.services();
    let target_session = srv.sessions.get(target_session_id).await?;

    let mut allowed = auth_session.can_see(&target_session);
    if !allowed {
        if let (Some(auth_user_id), Some(target_user_id)) =
            (auth_session.user_id(), target_session.user_id())
        {
            let target_user = srv.users.get(target_user_id, None).await?;
            if let Some(bot) = target_user.bot {
                if bot.owner_id == auth_user_id {
                    allowed = true;
                }
            }
        }
    }

    if !allowed {
        return Err(Error::NotFound);
    }

    if !json.changes(&target_session) {
        return Ok((StatusCode::NOT_MODIFIED, Json(target_session)));
    }
    data.session_update(target_session_id, json).await?;
    let srv = s.services();
    srv.sessions.invalidate(target_session_id).await;
    let target_session_new = srv.sessions.get(target_session_id).await?;
    s.broadcast(MessageSync::SessionUpdate {
        session: target_session_new.clone(),
    })?;
    if let Some(uid) = target_session_new.user_id() {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: uid.into_inner().into(),
            user_id: uid,
            session_id: Some(auth_session.id),
            reason,
            ty: AuditLogEntryType::SessionUpdate {
                session_id: target_session_new.id,
                changes: Changes::new()
                    .change("name", &target_session.name, &target_session_new.name)
                    .build(),
            },
        })
        .await?;
    }
    Ok((StatusCode::OK, Json(target_session_new)))
}

/// Session delete
#[utoipa::path(
    delete,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn session_delete(
    Path(target_session_id): Path<SessionIdReq>,
    AuthRelaxed(auth_session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let target_session_id = match target_session_id {
        SessionIdReq::SessionSelf => auth_session.id,
        SessionIdReq::SessionId(target_session_id) => target_session_id,
    };
    if auth_session.status == SessionStatus::Unauthorized && auth_session.id != target_session_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    let srv = s.services();
    let target_session = srv.sessions.get(target_session_id).await?;

    let mut allowed = auth_session.can_see(&target_session);
    if !allowed {
        if let (Some(auth_user_id), Some(target_user_id)) =
            (auth_session.user_id(), target_session.user_id())
        {
            let target_user = srv.users.get(target_user_id, None).await?;
            if let Some(bot) = target_user.bot {
                if bot.owner_id == auth_user_id {
                    allowed = true;
                }
            }
        }
    }

    if !allowed {
        return Err(Error::NotFound);
    }

    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete(target_session_id).await?;
    srv.sessions.invalidate(target_session_id).await;
    s.broadcast(MessageSync::SessionDelete {
        id: target_session_id,
        user_id: target_session.user_id(),
    })?;
    if let Some(uid) = auth_session.user_id() {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: uid.into_inner().into(),
            user_id: uid,
            session_id: Some(auth_session.id),
            reason,
            ty: AuditLogEntryType::SessionDelete {
                session_id: target_session_id,
            },
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Session delete all
///
/// Delete all sessions, *including the current one*
#[utoipa::path(
    delete,
    path = "/session/@all",
    tags = ["session"],
    responses((status = NO_CONTENT, description = "success")),
)]
pub async fn session_delete_all(
    AuthRelaxed(auth_session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let Some(user_id) = auth_session.user_id() else {
        return Err(Error::UnauthSession);
    };

    let data = s.data();
    let srv = s.services();

    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete_all(user_id).await?;
    srv.sessions.invalidate_all(user_id).await;
    s.broadcast(MessageSync::SessionDeleteAll { user_id })?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: user_id.into_inner().into(),
        user_id: user_id,
        session_id: Some(auth_session.id),
        reason,
        ty: AuditLogEntryType::SessionDeleteAll,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Session get
#[utoipa::path(
    get,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, body = Session, description = "success"),
    )
)]
pub async fn session_get(
    Path(session_id): Path<SessionIdReq>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let session_id = match session_id {
        SessionIdReq::SessionSelf => session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    let srv = s.services();
    let target_session = srv.sessions.get(session_id).await?;
    if !session.can_see(&target_session) {
        return Err(Error::NotFound);
    }
    Ok(Json(target_session))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(session_create))
        .routes(routes!(session_list))
        .routes(routes!(session_update))
        .routes(routes!(session_get))
        .routes(routes!(session_delete))
        .routes(routes!(session_delete_all))
}
