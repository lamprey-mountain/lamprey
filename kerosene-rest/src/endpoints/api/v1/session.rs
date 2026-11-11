use common::v1::types::{
    AuditLogEntryType, MessageSync, Session, SessionId, SessionToken, SessionType,
    SessionWithToken,
    misc::SessionIdReq,
    oauth::Scope,
    util::{Changes, Diff},
};
use http::StatusCode;
use lamprey_backend_core::error::Error as LegacyError;
use lamprey_backend_services::compat::types::DbSessionCreate;

use crate::prelude::*;

#[handler(routes::session_create)]
pub async fn create(
    req: Req<routes::session_create::Endpoint>,
) -> Result<routes::session_create::Response> {
    let body = req.inner();
    body.session
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let token = SessionToken::generate();
    let mut data = req.globals().begin().await.cast_internal()?;
    let session = data
        .session_create(DbSessionCreate {
            token: token.clone(),
            name: body.session.name.clone(),
            expires_at: None,
            ty: SessionType::User,
            application_id: None,
            ip_addr: None,
            user_agent: req.headers().user_agent.clone(),
        })
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    Ok(routes::session_create::Response {
        session: SessionWithToken { session, token },
    })
}

#[handler(routes::session_list)]
pub async fn list(
    req: Req<routes::session_list::Endpoint>,
) -> Result<routes::session_list::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let sessions = data
        .session_list(user.id, req.inner().pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::session_list::Response { sessions })
}

#[handler(routes::session_update)]
pub async fn update(
    req: Req<routes::session_update::Endpoint>,
) -> Result<routes::session_update::Response> {
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let target = target_session(&req, body.session_id.clone()).await?;
    if !body.patch.changes(&target) {
        return Ok(routes::session_update::Response {
            session: target,
            status: StatusCode::NOT_MODIFIED,
        });
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.session_update(target.id, body.patch.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.sessions.invalidate(target.id).await;
    let session = load_session(&req, target.id).await?;

    globals
        .messaging()
        .broadcast_global(MessageSync::SessionUpdate {
            session: session.clone(),
        })
        .await
        .cast_internal()?;

    if let Some(user_id) = session.user_id() {
        let actor_id = req.identity().user_id().unwrap_or(user_id);
        req.audit_log_as(actor_id, req.identity().ensure_session()?)
            .push(
                user_id.into_inner().into(),
                AuditLogEntryType::SessionUpdate {
                    session_id: session.id,
                    changes: Changes::new()
                        .change("name", &target.name, &session.name)
                        .build(),
                },
            )
            .success();
    }

    Ok(routes::session_update::Response {
        session,
        status: StatusCode::OK,
    })
}

#[handler(routes::session_delete)]
pub async fn delete(
    req: Req<routes::session_delete::Endpoint>,
) -> Result<routes::session_delete::Response> {
    let globals = req.globals();
    let srv = req.services();

    let target = target_session(&req, req.inner().session_id.clone()).await?;

    let mut data = globals.begin().await.cast_internal()?;
    data.session_delete(target.id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.sessions.invalidate(target.id).await;
    globals
        .messaging()
        .broadcast_global(MessageSync::SessionDelete {
            id: target.id,
            user_id: target.user_id(),
        })
        .await
        .cast_internal()?;

    if let Some(user_id) = target.user_id() {
        let actor_id = req.identity().user_id().unwrap_or(user_id);
        req.audit_log_as(actor_id, req.identity().ensure_session()?)
            .push(
                actor_id.into_inner().into(),
                AuditLogEntryType::SessionDelete {
                    session_id: target.id,
                    changes: Changes::new().remove("name", &target.name).build(),
                },
            )
            .success();
    }

    Ok(routes::session_delete::Response {})
}

#[handler(routes::session_delete_all)]
pub async fn delete_all(
    req: Req<routes::session_delete_all::Endpoint>,
) -> Result<routes::session_delete_all::Response> {
    let globals = req.globals();
    let srv = req.services();

    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let mut data = globals.begin().await.cast_internal()?;
    data.session_delete_all(user.id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.sessions.invalidate_all(user.id).await;
    globals
        .messaging()
        .broadcast_global(MessageSync::SessionDeleteAll { user_id: user.id })
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::SessionDeleteAll,
        )
        .success();

    Ok(routes::session_delete_all::Response {})
}

#[handler(routes::session_get)]
pub async fn get(req: Req<routes::session_get::Endpoint>) -> Result<routes::session_get::Response> {
    let session = target_session(&req, req.inner().session_id.clone()).await?;
    Ok(routes::session_get::Response { session })
}

/// load a session, returning [`ErrorCode::UnknownSession`] if it doesn't exist
async fn load_session<E: Endpoint>(req: &Req<E>, session_id: SessionId) -> Result<Session> {
    req.services()
        .sessions
        .get(session_id)
        .await
        .map(|session| (*session).clone())
        .map_err(|err| match err {
            LegacyError::NotFound => ApiError::from_code(ErrorCode::UnknownSession).into(),
            err => Error::internal(err),
        })
}

/// resolve the session this request targets
///
/// `@self` is allowed in any session state, everything else requires an
/// authenticated user that is allowed to see the target session
async fn target_session<E: Endpoint>(req: &Req<E>, target: SessionIdReq) -> Result<Session> {
    let identity = req.identity();
    let current = identity.ensure_session()?.clone();

    let target_id = match target {
        SessionIdReq::SessionSelf => return load_session(req, current.id).await,
        SessionIdReq::SessionId(session_id) => session_id,
    };

    identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let target = load_session(req, target_id).await?;
    if current.can_see(&target) || is_bot_owner(req, &target).await? {
        Ok(target)
    } else {
        Err(ApiError::from_code(ErrorCode::UnknownSession).into())
    }
}

/// whether the requesting user owns the bot this session belongs to
async fn is_bot_owner<E: Endpoint>(req: &Req<E>, target: &Session) -> Result<bool> {
    let Some(auth_user_id) = req.identity().user_id() else {
        return Ok(false);
    };
    let Some(target_user_id) = target.user_id() else {
        return Ok(false);
    };

    let target_user = req
        .services()
        .users
        .get(target_user_id, None)
        .await
        .cast_internal()?;
    if !target_user.bot {
        return Ok(false);
    }

    let app = req
        .globals()
        .begin_read()
        .await
        .cast_internal()?
        .application_get(target_user_id.into_inner().into())
        .await;

    Ok(app.is_ok_and(|app| app.owner_id == auth_user_id))
}

export_routes!(create, list, update, delete, delete_all, get);
