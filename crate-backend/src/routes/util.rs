use std::{sync::Arc, time::Duration};

use axum::{extract::FromRequestParts, http::request::Parts};
use common::v1::types::{application::Scope, application::Scopes, util::Time, SessionToken, User, UserId};
use headers::{authorization::Bearer, Authorization, HeaderMapExt};

use crate::{
    error::Error,
    types::{Session, SessionStatus, SessionType},
    ServerState,
};

/// extract authentication info for a request
// TODO: use this instead of the existing Auth stuff
pub struct Auth2 {
    /// the effective user making this request
    pub user: User,

    /// the real user making this request
    pub real_user: Option<User>,

    /// the session for this request
    pub session: Session,

    /// the oauth scopes this session has
    pub scopes: Scopes,
}

impl Auth2 {
    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<(), Error> {
        self.scopes.ensure_all(scopes).map_err(Into::into)
    }
}

/// extract the client's Session
pub struct AuthRelaxed(pub Session);

/// extract the client's Session iff it is authenticated
// TODO: remove
pub struct AuthWithSession(pub Session, pub User);

/// extract the client's Session iff it is authenticated and return the user
// TODO: remove
pub struct Auth(pub User);

/// extract the client's Session iff it is in sudo mode and return the user
pub struct AuthSudo(pub User);

/// extract the client's Session iff it is in sudo mode and return the session and user
pub struct AuthSudoWithSession(pub Session, pub User);

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
                let mut user = s.services().users.get(user_id, None).await?;
                if let Some(puppet_id) = puppet_id {
                    let puppet = s.services().users.get(puppet_id, None).await?;

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
                    if let Some(puppet) = &user.puppet {
                        let bot = s.services().users.get(puppet.owner_id, None).await?;

                        if let Some(bot) = &bot.bot {
                            let owner = s.services().users.get(bot.owner_id, None).await?;
                            user.suspended = owner.suspended;
                        }

                        user.suspended = bot.suspended;
                    } else if let Some(bot) = &user.bot {
                        let owner = s.services().users.get(bot.owner_id, None).await?;
                        user.suspended = owner.suspended;
                    }

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
                let user = s.services().users.get(user_id, None).await?;
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

impl FromRequestParts<Arc<ServerState>> for AuthSudoWithSession {
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
                let user = s.services().users.get(user_id, None).await?;
                Ok(Self(session, user))
            }
        }
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth2 {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthRelaxed(session) = AuthRelaxed::from_request_parts(parts, s).await?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;
        let srv = s.services();

        let HeaderPuppetId(puppet_id) = HeaderPuppetId::from_request_parts(parts, s).await?;
        let real_user = srv.users.get(user_id, None).await?;

        // load the real user if this is for puppeting
        let mut effective_user = if let Some(puppet_id) = puppet_id {
            let puppet = srv.users.get(puppet_id, None).await?;

            if let Some(bot) = &puppet.bot {
                if bot.owner_id == real_user.id {
                    puppet
                } else {
                    return Err(Error::BadStatic("not bot owner"));
                }
            } else {
                let Some(bot) = &real_user.bot else {
                    return Err(Error::BadStatic("user is not a bot"));
                };

                if !bot.is_bridge {
                    return Err(Error::BadStatic("bot is not a bridge"));
                }

                let Some(p) = &puppet.puppet else {
                    return Err(Error::BadStatic("can only puppet users of type Puppet"));
                };

                if p.owner_id != real_user.id {
                    return Err(Error::BadStatic("can only puppet your own puppets"));
                }

                puppet
            }
        } else {
            real_user.clone()
        };

        // propagate suspension
        if effective_user.id != real_user.id && real_user.is_suspended() {
            effective_user.suspended = real_user.suspended.clone();
        }

        if effective_user.suspended.is_none() {
            if let Some(puppet) = &effective_user.puppet {
                let bot = srv.users.get(puppet.owner_id, None).await?;
                if bot.is_suspended() {
                    effective_user.suspended = bot.suspended.clone();
                } else if let Some(bot_info) = &bot.bot {
                    let owner = srv.users.get(bot_info.owner_id, None).await?;
                    if owner.is_suspended() {
                        effective_user.suspended = owner.suspended.clone();
                    }
                }
            } else if let Some(bot) = &effective_user.bot {
                let owner = srv.users.get(bot.owner_id, None).await?;
                if owner.is_suspended() {
                    effective_user.suspended = owner.suspended.clone();
                }
            }
        }

        let scopes = if session.ty == SessionType::User {
            Scopes(vec![Scope::Auth])
        } else if let Some(app_id) = session.app_id {
            s.data()
                .connection_get(user_id, app_id)
                .await
                .map(|c| c.scopes)
                .unwrap_or_default()
        } else {
            Scopes::default()
        };

        Ok(Auth2 {
            user: effective_user,
            real_user: if puppet_id.is_some() {
                Some(real_user)
            } else {
                None
            },
            session,
            scopes,
        })
    }
}
