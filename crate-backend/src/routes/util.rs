use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};
use headers::{authorization::Bearer, Authorization, HeaderMapExt};
use http::HeaderValue;
use types::{SessionToken, UserId};

use crate::{
    error::Error,
    types::{Session, SessionStatus},
    ServerState,
};

pub struct AuthRelaxed(pub Session);
pub struct AuthWithSession(pub Session, pub UserId);
pub struct Auth(pub UserId);

/// extract the X-Reason header
pub struct HeaderReason(pub Option<String>);

/// extract the Idempotency-Key header
pub struct HeaderIdempotencyKey(pub Option<HeaderValue>);

impl FromRequestParts<Arc<ServerState>> for AuthRelaxed {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let auth: Authorization<Bearer> = parts
            .headers
            .typed_get()
            .ok_or_else(|| Error::MissingAuth)?;
        let session = s
            .data()
            .session_get_by_token(SessionToken(auth.token().to_string()))
            .await
            .map_err(|err| match err {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        Ok(Self(session))
    }
}

impl FromRequestParts<Arc<ServerState>> for AuthWithSession {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthRelaxed(session) = AuthRelaxed::from_request_parts(parts, s).await?;
        match session.status {
            SessionStatus::Unauthorized => Err(Error::UnauthSession),
            SessionStatus::Authorized { user_id } => Ok(Self(session, user_id)),
            SessionStatus::Sudo { user_id } => Ok(Self(session, user_id)),
        }
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthWithSession(_session, user_id) =
            AuthWithSession::from_request_parts(parts, s).await?;
        Ok(Self(user_id))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderReason {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("X-Reason")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());
        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderIdempotencyKey {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get("Idempotency-Key").cloned();
        Ok(Self(header))
    }
}
