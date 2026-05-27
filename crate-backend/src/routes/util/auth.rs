use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRequestParts;
use common::v1::types::federation::Hostname;
use common::v1::types::ids::SERVER_TOKEN_SESSION_ID;
use common::v1::types::oauth::Scope;
use common::v1::types::util::Time;
use common::v1::types::{oauth::Scopes, Session, User};
use common::v1::types::{RoomId, SessionImprint, SessionType, UserId, SERVER_USER_ID};
use common::v1::types::{SessionStatus, SessionToken};
use headers::authorization::Bearer;
use headers::{Authorization, HeaderMapExt};
use http::request::Parts;

use crate::routes::util::audit::AuditLoggerTransaction;
use crate::routes::util::{FederationIdentity, HeaderPuppetId, HeaderReason};
use crate::Error;
use crate::{routes::util::audit::AuditLogSlot, ServerState};

/// Empty scopes for Server/Public identities
static SCOPES_EMPTY: std::sync::LazyLock<Scopes> = std::sync::LazyLock::new(Scopes::default);

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

#[derive(Clone)]
pub enum AuthIdentity {
    Session {
        user: User,
        real_user: Option<User>,
        session: Session,
        scopes: Scopes,
    },
    Server {
        origin: Hostname,
        user: Option<User>,
    },
}

#[derive(Clone)]
pub struct Auth2 {
    pub identity: AuthIdentity,
    pub reason: Option<String>,
    pub audit_log_slot: Option<AuditLogSlot>,
    pub s: Arc<ServerState>,
}

#[derive(Clone)]
pub enum AuthIdentity3 {
    /// authenticated via a local user session
    Session {
        user: User,
        real_user: Option<User>,
        session: Session,
        scopes: Scopes,
    },

    /// authenticated via a remote server signature
    Server {
        origin: Hostname,

        /// the user the server is puppetting
        user: Option<User>,
    },

    /// unauthorized guest session (no user bound yet)
    Guest { session: Session, scopes: Scopes },

    /// truly public request (no authorization at all)
    Public,
}

#[derive(Clone)]
pub struct Auth3 {
    pub identity: AuthIdentity3,
    pub reason: Option<String>,
    pub audit_log_slot: Option<AuditLogSlot>,
    pub s: Arc<ServerState>,
}

impl Auth3 {
    pub fn user(&self) -> Result<&User, Error> {
        match &self.identity {
            AuthIdentity3::Session { user, .. } => Ok(user),
            AuthIdentity3::Server {
                user: Some(user), ..
            } => Ok(user),
            _ => Err(Error::MissingAuth),
        }
    }

    pub fn real_user(&self) -> Option<&User> {
        match &self.identity {
            AuthIdentity3::Session { real_user, .. } => real_user.as_ref(),
            AuthIdentity3::Server { .. } => None,
            _ => None,
        }
    }

    pub fn user_id(&self) -> Option<UserId> {
        match &self.identity {
            AuthIdentity3::Session { user, .. } => Some(user.id),
            AuthIdentity3::Server {
                user: Some(user), ..
            } => Some(user.id),
            _ => None,
        }
    }

    pub fn session(&self) -> Result<&Session, Error> {
        match &self.identity {
            AuthIdentity3::Session { session, .. } => Ok(session),
            AuthIdentity3::Guest { session, .. } => Ok(session),
            _ => Err(Error::MissingAuth),
        }
    }

    pub fn origin(&self) -> Result<&Hostname, Error> {
        match &self.identity {
            AuthIdentity3::Server { origin, .. } => Ok(origin),
            _ => Err(Error::MissingAuth),
        }
    }

    pub fn is_public(&self) -> bool {
        matches!(self.identity, AuthIdentity3::Public)
    }

    pub fn is_guest(&self) -> bool {
        matches!(self.identity, AuthIdentity3::Guest { .. })
    }

    pub fn scopes(&self) -> &Scopes {
        match &self.identity {
            AuthIdentity3::Session { scopes, .. } => scopes,
            AuthIdentity3::Guest { scopes, .. } => scopes,
            AuthIdentity3::Server { .. } => &SCOPES_EMPTY,
            AuthIdentity3::Public => &SCOPES_EMPTY,
        }
    }

    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<(), Error> {
        match &self.identity {
            AuthIdentity3::Session {
                scopes: self_scopes,
                ..
            } => self_scopes.ensure_all(scopes).map_err(Into::into),
            AuthIdentity3::Guest {
                scopes: self_scopes,
                ..
            } => self_scopes.ensure_all(scopes).map_err(Into::into),
            AuthIdentity3::Server { .. } => Ok(()),
            AuthIdentity3::Public => {
                if scopes.is_empty() {
                    Ok(())
                } else {
                    Err(Error::MissingAuth)
                }
            }
        }
    }

    pub fn ensure_sudo(&self) -> Result<(), Error> {
        match &self.identity {
            AuthIdentity3::Session { session, .. } => match &session.status {
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
            },
            AuthIdentity3::Server { .. } => Ok(()), // servers are sudo?
            _ => Err(Error::MissingAuth),
        }
    }

    pub fn audit_log(&self, context_id: RoomId) -> Result<AuditLoggerTransaction, Error> {
        let user = self.user()?.clone();
        let session = self.session()?.clone();
        let real_user = self.real_user().cloned();

        Ok(AuditLoggerTransaction {
            context_id,
            auth: Auth {
                user,
                real_user,
                session,
                scopes: self.scopes().clone(),
                reason: self.reason.clone(),
                audit_log_slot: self.audit_log_slot.clone(),
                s: Arc::clone(&self.s),
            },
            reason: self.reason.clone(),
            started_at: Time::now_utc(),
            application_id: None,
            ty: None,
            status: None,
        })
    }
}

pub struct Auth3Relaxed {
    pub auth: Auth3,
}

impl FromRequestParts<Arc<ServerState>> for Auth3 {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        // try session auth
        if parts.headers.contains_key(http::header::AUTHORIZATION) {
            let relaxed = AuthRelaxed2::from_request_parts(parts, s).await?;
            let identity = if let Some(user) = relaxed.user {
                AuthIdentity3::Session {
                    user,
                    real_user: relaxed.real_user,
                    session: relaxed.session,
                    scopes: relaxed.scopes,
                }
            } else {
                AuthIdentity3::Guest {
                    session: relaxed.session,
                    scopes: relaxed.scopes,
                }
            };

            return Ok(Auth3 {
                identity,
                reason: relaxed.reason,
                audit_log_slot: relaxed.audit_log_slot,
                s: Arc::clone(&relaxed.s),
            });
        }

        // try federation auth
        let federation_identity = parts.extensions.get::<FederationIdentity>().cloned();
        if let Some(FederationIdentity(origin)) = federation_identity {
            let HeaderPuppetId(puppet_id) = HeaderPuppetId::from_request_parts(parts, s).await?;
            let user = if let Some(_puppet_id) = puppet_id {
                // let user = s.services().users.get(puppet_id, None).await?;
                // Some(user)

                // TODO: user puppetting
                // verify puppet belongs to this server
                return Err(Error::Unimplemented);
            } else {
                None
            };

            return Ok(Auth3 {
                identity: AuthIdentity3::Server {
                    origin: origin.clone(),
                    user,
                },
                reason: HeaderReason::from_request_parts(parts, s).await?.0,
                audit_log_slot: parts.extensions.get::<AuditLogSlot>().cloned(),
                s: Arc::clone(s),
            });
        }

        Ok(Auth3 {
            identity: AuthIdentity3::Public,
            reason: HeaderReason::from_request_parts(parts, s).await?.0,
            audit_log_slot: parts.extensions.get::<AuditLogSlot>().cloned(),
            s: Arc::clone(s),
        })
    }
}

impl Auth2 {
    pub fn user(&self) -> Result<&User, Error> {
        match &self.identity {
            AuthIdentity::Session { user, .. } => Ok(user),
            AuthIdentity::Server {
                user: Some(user), ..
            } => Ok(user),
            AuthIdentity::Server { user: None, .. } => Err(Error::MissingAuth),
        }
    }

    pub fn origin(&self) -> Result<&Hostname, Error> {
        match &self.identity {
            AuthIdentity::Session { .. } => Err(Error::MissingAuth),
            AuthIdentity::Server { origin, .. } => Ok(origin),
        }
    }

    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<(), Error> {
        match &self.identity {
            AuthIdentity::Session {
                scopes: self_scopes,
                ..
            } => self_scopes.ensure_all(scopes).map_err(Into::into),
            AuthIdentity::Server { .. } => Ok(()), // TODO: server scopes
        }
    }

    pub fn ensure_sudo(&self) -> Result<(), Error> {
        match &self.identity {
            AuthIdentity::Session { session, .. } => match &session.status {
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
            },
            AuthIdentity::Server { .. } => Ok(()), // servers are sudo?
        }
    }
}

pub struct Auth2Relaxed {
    pub identity: Option<AuthIdentity>,
    pub reason: Option<String>,
    pub audit_log_slot: Option<AuditLogSlot>,
    pub s: Arc<ServerState>,
}

impl Auth2Relaxed {
    pub fn upgrade(self) -> Result<Auth2, Error> {
        let identity = self.identity.ok_or(Error::MissingAuth)?;
        Ok(Auth2 {
            identity,
            reason: self.reason,
            audit_log_slot: self.audit_log_slot,
            s: self.s,
        })
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth2Relaxed {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        // try session auth
        if parts.headers.contains_key(http::header::AUTHORIZATION) {
            let res = AuthRelaxed2::from_request_parts(parts, s).await;
            if let Ok(relaxed) = res {
                if let Some(user) = relaxed.user {
                    return Ok(Auth2Relaxed {
                        identity: Some(AuthIdentity::Session {
                            user,
                            real_user: relaxed.real_user,
                            session: relaxed.session,
                            scopes: relaxed.scopes,
                        }),
                        reason: relaxed.reason,
                        audit_log_slot: relaxed.audit_log_slot,
                        s: Arc::clone(&relaxed.s),
                    });
                }
            }
        }

        // try federation auth
        let federation_identity = parts.extensions.get::<FederationIdentity>().cloned();
        if let Some(FederationIdentity(origin)) = federation_identity {
            let HeaderPuppetId(puppet_id) = HeaderPuppetId::from_request_parts(parts, s).await?;
            let user = if let Some(puppet_id) = puppet_id {
                let user = s.services().users.get(puppet_id, None).await?;
                // TODO: verify puppet belongs to this server
                Some(user)
            } else {
                None
            };

            return Ok(Auth2Relaxed {
                identity: Some(AuthIdentity::Server {
                    origin: origin.clone(),
                    user,
                }),
                reason: HeaderReason::from_request_parts(parts, s).await?.0,
                audit_log_slot: parts.extensions.get::<AuditLogSlot>().cloned(),
                s: Arc::clone(s),
            });
        }

        Ok(Auth2Relaxed {
            identity: None,
            reason: HeaderReason::from_request_parts(parts, s).await?.0,
            audit_log_slot: parts.extensions.get::<AuditLogSlot>().cloned(),
            s: Arc::clone(s),
        })
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth2 {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let relaxed = Auth2Relaxed::from_request_parts(parts, s).await?;
        relaxed.upgrade()
    }
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
                imprint: SessionImprint {
                    last_seen_at: Time::now_utc(),
                    ip_addr: None,
                    country_code: None,
                    country_name: None,
                    city_name: None,
                    user_agent: None,
                },
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
                s: Arc::clone(s),
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
        if session.imprint.last_seen_at < Time::now_utc() - Duration::from_secs(60) {
            let user_agent = parts
                .headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // get the first ip in the chain
            let ip_addr: Option<IpAddr> = parts
                .headers
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim())
                .and_then(|s| s.parse().ok());

            let geo = ip_addr.and_then(|ip| srv.ips.lookup(ip).ok().flatten());

            s.data()
                .session_update_imprint(
                    session.id,
                    SessionImprint {
                        last_seen_at: Time::now_utc(),
                        ip_addr: ip_addr.map(|i| i.to_string()),
                        country_code: geo.as_ref().and_then(|g| g.country_code.clone()),
                        country_name: geo.as_ref().and_then(|g| g.country_name.clone()),
                        city_name: geo.as_ref().and_then(|g| g.city_name.clone()),
                        user_agent,
                    },
                )
                .await?;
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
            s: Arc::clone(s),
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
