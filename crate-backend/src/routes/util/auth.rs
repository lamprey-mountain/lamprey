use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRequestParts;
use common::v1::types::oauth::Scope;
use common::v1::types::util::Time;
use common::v1::types::{oauth::Scopes, Session, User};
use common::v1::types::{SessionStatus, SessionToken, SessionType};
use common::v1::types::{SERVER_TOKEN_SESSION_ID, SERVER_USER_ID};
use headers::authorization::Bearer;
use headers::{Authorization, HeaderMapExt};
use http::request::Parts;

use crate::routes::util::{HeaderPuppetId, HeaderReason};
use crate::Error;
use crate::{routes::util::audit::AuditLogSlot, ServerState};

/// extract authentication info for a request
#[derive(Clone)]
pub struct Auth {
    /// the effective user making this request
    pub user: User,

    /// the real user making this request
    pub real_user: Option<User>,

    /// the session for this request
    pub session: Session,

    /// the oauth scopes this session has
    pub scopes: Scopes,

    /// the audit log reason for this request
    ///
    /// extracted from HeaderReason
    pub reason: Option<String>,

    /// the audit log slot for this request
    pub audit_log_slot: Option<AuditLogSlot>,

    /// a reference to the server state
    pub s: Arc<ServerState>,
}

impl Auth {
    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<(), Error> {
        self.scopes.ensure_all(scopes).map_err(Into::into)
    }

    pub fn ensure_sudo(&self) -> Result<(), Error> {
        match &self.session.status {
            SessionStatus::Unauthorized => Err(Error::UnauthSession),
            SessionStatus::Bound { .. } => Err(Error::UnauthSession),
            SessionStatus::Authorized { .. } => Err(Error::BadStatic("needs sudo")),
            SessionStatus::Sudo {
                sudo_expires_at, ..
            } => {
                if *sudo_expires_at < Time::now_utc() {
                    Err(Error::BadStatic("sudo session expired"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

pub struct AuthRelaxed2 {
    /// the effective user making this request (may be uninitialized for unauthorized sessions)
    pub user: Option<User>,

    /// the real user making this request (for puppeting)
    pub real_user: Option<User>,

    /// the session for this request
    pub session: Session,

    /// the oauth scopes this session has
    pub scopes: Scopes,

    /// the audit log reason for this request
    ///
    /// extracted from HeaderReason
    pub reason: Option<String>,

    /// the audit log slot for this request
    pub audit_log_slot: Option<AuditLogSlot>,

    /// a reference to the server state
    pub s: Arc<ServerState>,
}

impl AuthRelaxed2 {
    pub fn upgrade(self) -> Result<Auth, Error> {
        let user = self.user.ok_or(Error::UnauthSession)?;
        Ok(Auth {
            user,
            real_user: self.real_user,
            session: self.session,
            scopes: self.scopes,
            reason: self.reason,
            audit_log_slot: self.audit_log_slot,
            s: self.s,
        })
    }

    /// Ensure this session has an associated user
    pub fn ensure_has_user(&self) -> Result<&User, Error> {
        self.user.as_ref().ok_or_else(|| Error::UnauthSession)
    }

    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<(), Error> {
        self.scopes.ensure_all(scopes).map_err(Into::into)
    }

    pub fn ensure_sudo(&self) -> Result<(), Error> {
        match &self.session.status {
            SessionStatus::Unauthorized => Err(Error::UnauthSession),
            SessionStatus::Bound { .. } => Err(Error::UnauthSession),
            SessionStatus::Authorized { .. } => Err(Error::BadStatic("needs sudo")),
            SessionStatus::Sudo {
                sudo_expires_at, ..
            } => {
                if *sudo_expires_at < Time::now_utc() {
                    Err(Error::BadStatic("sudo session expired"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl FromRequestParts<Arc<ServerState>> for AuthRelaxed2 {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let auth: Authorization<Bearer> = parts
            .headers
            .typed_get()
            .ok_or_else(|| Error::MissingAuth)?;
        let token = auth.token();
        let srv = s.services();

        // check admin token
        if srv.admin.verify_admin_token(token).await {
            let user = srv.users.get(SERVER_USER_ID, None).await?;
            let session = Session {
                id: SERVER_TOKEN_SESSION_ID,
                status: SessionStatus::Sudo {
                    user_id: SERVER_USER_ID,
                    sudo_expires_at: Time::now_utc() + Duration::from_secs(3600),
                },
                name: Some("admin token".to_string()),
                ty: SessionType::User,
                expires_at: None,
                app_id: None,
                last_seen_at: Time::now_utc(),
                ip_addr: None,
                user_agent: None,
                authorized_at: Some(Time::now_utc()),
                deauthorized_at: None,
            };

            return Ok(AuthRelaxed2 {
                user: Some(user),
                real_user: None,
                session,
                scopes: Scopes(vec![Scope::Full]),
                reason: HeaderReason::from_request_parts(parts, s).await?.0,
                audit_log_slot: parts.extensions.get::<AuditLogSlot>().cloned(),
                s: s.clone(),
            });
        }

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
        if matches!(session.status, SessionStatus::Bound { .. }) {
            return Err(Error::MissingAuth);
        }

        let reason = HeaderReason::from_request_parts(parts, s).await?;

        // Try to get user info if session is authorized
        let (user, real_user) = match session.user_id() {
            Some(user_id) => {
                let real_user = srv.users.get(user_id, None).await?;

                let HeaderPuppetId(puppet_id) =
                    HeaderPuppetId::from_request_parts(parts, s).await?;

                let effective_user = if let Some(puppet_id) = puppet_id {
                    let puppet = srv.users.get(puppet_id, None).await?;

                    if puppet.bot {
                        let app = s
                            .data()
                            .application_get(puppet.id.into_inner().into())
                            .await?;
                        if app.owner_id == real_user.id {
                            puppet
                        } else {
                            return Err(Error::BadStatic("not bot owner"));
                        }
                    } else {
                        if !real_user.bot {
                            return Err(Error::BadStatic("user is not a bot"));
                        }

                        let Some(p) = &puppet.puppet else {
                            return Err(Error::BadStatic("can only puppet users of type Puppet"));
                        };

                        if p.owner_id.into_inner() != *real_user.id {
                            return Err(Error::BadStatic("can only puppet your own puppets"));
                        }

                        puppet
                    }
                } else {
                    real_user.clone()
                };

                // Propagate suspension
                let mut final_user = effective_user;
                if final_user.id != real_user.id && real_user.is_suspended() {
                    final_user.suspended = real_user.suspended.clone();
                }

                if final_user.suspended.is_none() {
                    if let Some(puppet) = &final_user.puppet {
                        let bot_app_id = puppet.owner_id;
                        let bot_user = srv.users.get(bot_app_id.into_inner().into(), None).await?;
                        if bot_user.is_suspended() {
                            final_user.suspended = bot_user.suspended.clone();
                        } else if bot_user.bot {
                            if let Ok(app) = s.data().application_get(bot_app_id).await {
                                let owner = srv.users.get(app.owner_id, None).await?;
                                if owner.is_suspended() {
                                    final_user.suspended = owner.suspended.clone();
                                }
                            }
                        }
                    } else if final_user.bot {
                        if let Ok(app) = s
                            .data()
                            .application_get(final_user.id.into_inner().into())
                            .await
                        {
                            let owner = srv.users.get(app.owner_id, None).await?;
                            if owner.is_suspended() {
                                final_user.suspended = owner.suspended.clone();
                            }
                        }
                    }
                }

                (
                    Some(final_user),
                    if puppet_id.is_some() {
                        Some(real_user)
                    } else {
                        None
                    },
                )
            }
            None => (None, None),
        };

        let scopes = if session.ty == SessionType::User {
            Scopes(vec![Scope::Auth])
        } else if let Some(app_id) = session.app_id {
            s.data()
                .connection_get(session.user_id().unwrap_or(SERVER_USER_ID), app_id)
                .await
                .map(|c| c.scopes)
                .unwrap_or_default()
        } else {
            Scopes::default()
        };

        let audit_log_slot = parts.extensions.get::<AuditLogSlot>().cloned();

        Ok(AuthRelaxed2 {
            user,
            real_user,
            session,
            scopes,
            reason: reason.0,
            audit_log_slot,
            s: s.clone(),
        })
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let relaxed = AuthRelaxed2::from_request_parts(parts, s).await?;
        relaxed.upgrade()
    }
}
