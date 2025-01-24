use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use types::{PaginationQuery, PaginationResponse, Session, SessionCreate, SessionId, SessionPatch, SessionStatus};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::SessionIdReq;
use crate::ServerState;

use super::util::{Auth, AuthRelaxed};
use crate::error::{Error, Result};

/// Session create
#[utoipa::path(
    post,
    path = "/session",
    tags = ["session"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn session_create(
    State(s): State<Arc<ServerState>>,
    Json(body): Json<SessionCreate>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let session = data.session_create(body.user_id, body.name).await?;
    Ok((StatusCode::CREATED, Json(session)))
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
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.session_list(session.user_id, q).await?;
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
    Json(patch): Json<SessionPatch>,
) -> Result<impl IntoResponse> {
    let session_id = match session_id {
        SessionIdReq::SessionSelf => session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    if session.status == SessionStatus::Unauthorized && session.id != session_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    let target_session = data.session_get(session_id).await?;
    if target_session.user_id != session.user_id {
        return Err(Error::NotFound);
    }
    if patch.wont_change(&session) {
        return Ok((StatusCode::NOT_MODIFIED, Json(session)));
    }
    data.session_update(session_id, patch).await?;
    let session = data.session_get(session_id).await?;
    s.broadcast(types::MessageSync::UpsertSession { session: session.clone() })?;
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
    let target_session = data.session_get(session_id).await?;
    if target_session.user_id != session.user_id {
        return Err(Error::NotFound);
    }
    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete(session_id).await?;
    s.broadcast(types::MessageSync::DeleteSession { id: session_id })?;
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
    if session.status == SessionStatus::Unauthorized && session.id != session_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    let session = data.session_get(session_id).await?;
    Ok(Json(session))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(session_create))
        .routes(routes!(session_list))
        .routes(routes!(session_update))
        .routes(routes!(session_get))
        .routes(routes!(session_delete))
}
