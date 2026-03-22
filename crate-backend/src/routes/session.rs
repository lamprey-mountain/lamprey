use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntryType, MessageSync, SessionStatus, SessionToken, SessionType, SessionWithToken,
};
use lamprey_macros::handler;
use uuid::Uuid;
use validator::Validate;

use crate::routes::util::Auth;
use crate::types::{DbSessionCreate, SessionIdReq};
use crate::{routes2, ServerState};
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};

/// Session create
#[handler(routes::session_create)]
pub async fn session_create(
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    req: routes::session_create::Request,
) -> Result<impl IntoResponse> {
    let json = req.session;
    json.validate()?;
    let data = s.data();
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let token = SessionToken(Uuid::new_v4().to_string()); // TODO: is this secure enough
    let session = data
        .session_create(DbSessionCreate {
            token: token.clone(),
            name: json.name,
            expires_at: None,
            ty: SessionType::User,
            application_id: None,
            ip_addr: None,
            user_agent,
        })
        .await?;
    let session_with_token = SessionWithToken { session, token };
    Ok((StatusCode::CREATED, Json(session_with_token)))
}

/// Session list
#[handler(routes::session_list)]
pub async fn session_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::session_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let res = data.session_list(auth.user.id, req.pagination).await?;
    Ok(Json(res))
}

/// Session update
#[handler(routes::session_update)]
pub async fn session_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::session_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let json = req.patch;
    json.validate()?;
    let target_session_id = match req.session_id {
        SessionIdReq::SessionSelf => auth.session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    let data = s.data();
    let srv = s.services();
    let target_session = srv.sessions.get(target_session_id).await?;

    let mut allowed = auth.session.can_see(&target_session);
    if !allowed {
        if let (Some(auth_user_id), Some(target_user_id)) =
            (auth.session.user_id(), target_session.user_id())
        {
            let target_user = srv.users.get(target_user_id, None).await?;
            if target_user.bot {
                if let Ok(app) = s
                    .data()
                    .application_get(target_user_id.into_inner().into())
                    .await
                {
                    if app.owner_id == auth_user_id {
                        allowed = true;
                    }
                }
            }
        }
    }

    if !allowed {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownSession,
        )));
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
        let al = auth.audit_log(uid.into_inner().into());
        al.commit_success(AuditLogEntryType::SessionUpdate {
            session_id: target_session_new.id,
            changes: Changes::new()
                .change("name", &target_session.name, &target_session_new.name)
                .build(),
        })
        .await?;
    }
    Ok((StatusCode::OK, Json(target_session_new)))
}

/// Session delete
#[handler(routes::session_delete)]
pub async fn session_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::session_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let target_session_id = match req.session_id {
        SessionIdReq::SessionSelf => auth.session.id,
        SessionIdReq::SessionId(target_session_id) => target_session_id,
    };
    if auth.session.status == SessionStatus::Unauthorized && auth.session.id != target_session_id {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownSession,
        )));
    }
    let data = s.data();
    let srv = s.services();
    let target_session = srv.sessions.get(target_session_id).await?;

    let mut allowed = auth.session.can_see(&target_session);
    if !allowed {
        if let (Some(auth_user_id), Some(target_user_id)) =
            (auth.session.user_id(), target_session.user_id())
        {
            let target_user = srv.users.get(target_user_id, None).await?;
            if target_user.bot {
                if let Ok(app) = s
                    .data()
                    .application_get(target_user_id.into_inner().into())
                    .await
                {
                    if app.owner_id == auth_user_id {
                        allowed = true;
                    }
                }
            }
        }
    }

    if !allowed {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownSession,
        )));
    }

    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete(target_session_id).await?;
    srv.sessions.invalidate(target_session_id).await;
    s.broadcast(MessageSync::SessionDelete {
        id: target_session_id,
        user_id: target_session.user_id(),
    })?;
    if let Some(uid) = auth.session.user_id() {
        let al = auth.audit_log(uid.into_inner().into());
        al.commit_success(AuditLogEntryType::SessionDelete {
            session_id: target_session_id,
            changes: Changes::new().remove("name", &target_session.name).build(),
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Session delete all
///
/// Delete all sessions, *including the current one*
#[handler(routes::session_delete_all)]
pub async fn session_delete_all(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::session_delete_all::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let Some(user_id) = auth.session.user_id() else {
        return Err(Error::UnauthSession);
    };

    let data = s.data();
    let srv = s.services();

    // TODO: should i restrict deleting other sessions to sudo mode?
    data.session_delete_all(user_id).await?;
    srv.sessions.invalidate_all(user_id).await;
    s.broadcast(MessageSync::SessionDeleteAll { user_id })?;
    let al = auth.audit_log(user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::SessionDeleteAll)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Session get
#[handler(routes::session_get)]
pub async fn session_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::session_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let session_id = match req.session_id {
        SessionIdReq::SessionSelf => auth.session.id,
        SessionIdReq::SessionId(session_id) => session_id,
    };
    let srv = s.services();
    let target_session = srv.sessions.get(session_id).await?;
    if !auth.session.can_see(&target_session) {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownSession,
        )));
    }
    Ok(Json(target_session))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(session_create))
        .routes(routes2!(session_list))
        .routes(routes2!(session_update))
        .routes(routes2!(session_get))
        .routes(routes2!(session_delete))
        .routes(routes2!(session_delete_all))
}
