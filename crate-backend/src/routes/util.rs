use std::{sync::Arc, time::Duration};

use axum::{extract::FromRequestParts, http::request::Parts};
use common::v1::types::{util::Time, SessionToken, User, UserId};
use headers::{authorization::Bearer, Authorization, HeaderMapExt};

use crate::{
    error::Error,
    types::{Session, SessionStatus},
    ServerState,
};

/// extract the client's Session
pub struct AuthRelaxed(pub Session);

/// extract the client's Session iff it is authenticated
pub struct AuthWithSession(pub Session, pub User);

/// extract the client's Session iff it is authenticated and return the user_id
pub struct Auth(pub User);

/// extract the client's Session iff it is in sudo mode and return the user_id
pub struct AuthSudo(pub User);

/// extract the X-Reason header
pub struct HeaderReason(pub Option<String>);

/// extract the Idempotency-Key header
pub struct HeaderIdempotencyKey(pub Option<String>);

/// extract the X-Puppet-Id header
pub struct HeaderPuppetId(pub Option<UserId>);

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
        let srv = s.services();
        let session = srv
            .sessions
            .get_by_token(SessionToken(auth.token().to_string()))
            .await
            .map_err(|err| match err {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        if session.expires_at.is_some_and(|t| t < Time::now_utc()) {
            return Err(Error::MissingAuth);
        }
        if session.last_seen_at < Time::now_utc() - Duration::from_secs(60) {
            s.data().session_set_last_seen_at(session.id).await?;
            srv.sessions.invalidate(session.id).await;
        }
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
            SessionStatus::Authorized { user_id } | SessionStatus::Sudo { user_id, .. } => {
                let HeaderPuppetId(puppet_id) =
                    HeaderPuppetId::from_request_parts(parts, s).await?;
                let user = s.services().users.get(user_id).await?;
                if let Some(puppet_id) = puppet_id {
                    let puppet = s.services().users.get(puppet_id).await?;

                    if let Some(bot) = &puppet.bot {
                        if bot.owner_id == user.id {
                            return Ok(Self(session, puppet));
                        }
                    }

                    let Some(bot) = user.bot else {
                        return Err(Error::BadStatic("user is not a bot"));
                    };

                    if !bot.is_bridge {
                        return Err(Error::BadStatic("bot is not a bridge"));
                    }

                    let Some(p) = &puppet.puppet else {
                        return Err(Error::BadStatic("can only puppet users of type Puppet"));
                    };

                    if p.owner_id != user.id {
                        return Err(Error::BadStatic("can only puppet your own puppets"));
                    }

                    Ok(Self(session, puppet))
                } else {
                    Ok(Self(session, user))
                }
            }
        }
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthWithSession(_session, user) = AuthWithSession::from_request_parts(parts, s).await?;
        Ok(Self(user))
    }
}

impl FromRequestParts<Arc<ServerState>> for AuthSudo {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthRelaxed(session) = AuthRelaxed::from_request_parts(parts, s).await?;
        match session.status {
            SessionStatus::Unauthorized => Err(Error::UnauthSession),
            SessionStatus::Authorized { .. } => Err(Error::BadStatic("needs sudo")),
            SessionStatus::Sudo { user_id, .. } => {
                let user = s.services().users.get(user_id).await?;
                Ok(Self(user))
            }
        }
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

        if let Some(ref reason) = header {
            if reason.chars().count() > 1024 {
                return Err(Error::BadRequest(
                    "X-Audit-Reason must be 1024 characters or less".to_string(),
                ));
            }
        }

        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderIdempotencyKey {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("Idempotency-Key")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());
        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderPuppetId {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let puppet_id = parts
            .headers
            .get("X-Puppet-Id")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.parse().ok());
        Ok(Self(puppet_id))
    }
}
