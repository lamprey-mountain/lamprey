use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Diff;
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, Session, SessionCreate, SessionId,
    SessionPatch, SessionStatus, SessionToken, SessionWithToken,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::types::SessionIdReq;
use crate::ServerState;

use super::util::{AuthRelaxed, AuthWithSession};
use crate::error::{Error, Result};

// TODO: expire old unused sessions
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
    let session = data.session_create(token.clone(), json.name, None).await?;
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
    AuthWithSession(_session, user_id): AuthWithSession,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.session_list(user_id, q).await?;
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
    Path(session_id): Path<SessionIdReq>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<SessionPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let session_id = match session_id {
        SessionIdReq::SessionSelf => session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    let data = s.data();
    let srv = s.services();
    let target_session = srv.sessions.get(session_id).await?;
    if !session.can_see(&target_session) {
        return Err(Error::NotFound);
    }
    if !json.changes(&session) {
        return Ok((StatusCode::NOT_MODIFIED, Json(session)));
    }
    data.session_update(session_id, json).await?;
    let srv = s.services();
    srv.sessions.invalidate(session_id).await;
    let session = srv.sessions.get(session_id).await?;
    s.broadcast(MessageSync::SessionUpdate {
        session: session.clone(),
    })?;
    Ok((StatusCode::OK, Json(session)))
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
    Path(session_id): Path<SessionIdReq>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let session_id = match session_id {
        SessionIdReq::SessionSelf => session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    if session.status == SessionStatus::Unauthorized && session.id != session_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    let srv = s.services();
    let target_session = srv.sessions.get(session_id).await?;
    if !session.can_see(&target_session) {
        return Err(Error::NotFound);
    }
    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete(session_id).await?;
    srv.sessions.invalidate(session_id).await;
    s.broadcast(MessageSync::SessionDelete {
        id: session_id,
        user_id: target_session.user_id(),
    })?;
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
}
