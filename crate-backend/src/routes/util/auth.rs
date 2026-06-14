use core::fmt;
use std::{net::IpAddr, sync::Arc, time::Duration};

use crate::{
    ServerState,
    prelude::*,
    routes::util::{FederationIdentity, audit::AuditTxnSlot, headers::HeadersRequest},
};
use axum::extract::FromRequestParts;
use common::v1::types::{
    Session, SessionImprint, SessionStatus, SessionToken, SessionType, User, UserId,
    error::{ApiError, ErrorCode},
    federation::Hostname,
    ids::{SERVER_TOKEN_SESSION_ID, SERVER_USER_ID},
    oauth::{Scope, Scopes},
    util::Time,
};
use http::request::Parts;

#[derive(Clone)]
pub struct Auth4 {
    identity: Identity,

    // TEMP: make begin_audit_log work
    pub(super) reason: Option<String>,
    pub(super) audit_txn_slot: AuditTxnSlot,
}

// TEMP: AuditTxnSlot is not Debug
impl fmt::Debug for Auth4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Auth4")
            .field("identity", &self.identity)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum Identity {
    /// a user's session
    User {
        user: User,
        session: Session,
        scopes: Scopes,
    },

    /// an oauth application acting on behalf of a user
    Oauth { user: User, scopes: Scopes },

    /// a bridge application controlling one of their puppets
    Puppet {
        puppet: User,
        puppeteer: User,

        /// the puppeteer's session
        session: Session,

        // how do these work?
        scopes: Scopes,
    },

    /// authenticated via a remote server signature
    Server {
        hostname: Hostname,

        /// the user the server is puppetting
        puppet: Option<User>,
    },

    /// unauthorized guest session (no user bound yet)
    Guest { session: Session, scopes: Scopes },

    /// truly public request (no authorization at all)
    Public,
}

impl Auth4 {
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// get the acting user
    ///
    /// for puppet/server, returns the puppeted user
    pub fn user(&self) -> Option<&User> {
        match &self.identity {
            Identity::User { user, .. } => Some(user),
            Identity::Oauth { user, .. } => Some(user),
            Identity::Puppet { puppet, .. } => Some(puppet),
            Identity::Server {
                puppet: Some(puppet),
                ..
            } => Some(puppet),
            _ => None,
        }
    }

    pub fn user_id(&self) -> Option<UserId> {
        self.user().map(|u| u.id)
    }

    /// attempt to get the session, if known
    pub fn session(&self) -> Option<&Session> {
        match &self.identity {
            Identity::User { session, .. } => Some(session),
            Identity::Guest { session, .. } => Some(session),
            Identity::Puppet { session, .. } => Some(session),
            _ => None,
        }
    }

    pub fn scopes(&self) -> Option<&Scopes> {
        match &self.identity {
            Identity::User { scopes, .. } => Some(scopes),
            Identity::Oauth { scopes, .. } => Some(scopes),
            Identity::Guest { scopes, .. } => Some(scopes),
            Identity::Puppet { scopes, .. } => Some(scopes),
            _ => None,
        }
    }

    pub fn origin(&self) -> Option<&Hostname> {
        match &self.identity {
            Identity::Server { hostname, .. } => Some(hostname),
            _ => None,
        }
    }

    /// like `self.user()` but returns an error instead of `None`
    pub fn ensure_user(&self) -> Result<&User> {
        self.user().ok_or(Error::MissingAuth)
    }

    pub fn ensure_session(&self) -> Result<&Session> {
        self.session().ok_or(Error::MissingAuth)
    }

    pub fn ensure_origin(&self) -> Result<&Hostname> {
        self.origin().ok_or(Error::MissingAuth)
    }

    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<()> {
        let self_scopes = self.scopes().ok_or(Error::MissingAuth)?;
        self_scopes.ensure_all(scopes).map_err(Into::into)
    }

    pub fn ensure_sudo(&self) -> Result<()> {
        // servers are always sudo
        if let Identity::Server { .. } = &self.identity {
            return Ok(());
        }

        let session = self.session().ok_or(Error::MissingAuth)?;
        match &session.status {
            SessionStatus::Sudo {
                sudo_expires_at, ..
            } => {
                if *sudo_expires_at < Time::now_utc() {
                    Err(Error::ApiError(ApiError::from_code(
                        ErrorCode::SudoSessionExpired,
                    )))
                } else {
                    Ok(())
                }
            }
            _ => Err(Error::ApiError(ApiError::from_code(
                ErrorCode::SudoRequired,
            ))),
        }
    }
}

impl Auth4 {
    pub async fn calculate(parts: &Parts, state: &ServerState) -> Result<Self> {
        let srv = state.services();

        // TEMP: make begin_audit_log work
        let headers = HeadersRequest::from_parts(parts)?;
        let reason = headers.reason;
        let audit_txn: &AuditTxnSlot = parts.extensions.get().expect("always exists");
        let audit_txn_slot = Arc::clone(audit_txn);

        // bearer authorization token
        if let Some(auth) = &headers.authorization {
            let token = auth.token();

            // admin token
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

                return Ok(Auth4 {
                    identity: Identity::User {
                        user,
                        session,
                        scopes: Scopes(vec![Scope::Full]),
                    },
                    reason,
                    audit_txn_slot,
                });
            }

            // user session
            let session = srv
                .sessions
                .get_by_token(SessionToken(token.to_string()))
                .await?;
            if session.expires_at.is_some_and(|t| t < Time::now_utc()) {
                return Err(Error::MissingAuth);
            }

            // session imprint update
            if session.imprint.last_seen_at < Time::now_utc() - Duration::from_secs(60) {
                let geo = headers
                    .ip_addr
                    .and_then(|ip| srv.ips.lookup(ip).ok().flatten());

                state
                    .data()
                    .session_update_imprint(
                        session.id,
                        SessionImprint {
                            last_seen_at: Time::now_utc(),
                            ip_addr: headers.ip_addr.map(|i| i.to_string()),
                            country_code: geo.as_ref().and_then(|g| g.country_code.clone()),
                            country_name: geo.as_ref().and_then(|g| g.country_name.clone()),
                            city_name: geo.as_ref().and_then(|g| g.city_name.clone()),
                            user_agent: headers.user_agent,
                        },
                    )
                    .await?;
            }

            let real_user = if let Some(user_id) = session.user_id() {
                Some(srv.users.get(user_id, None).await?)
            } else {
                None
            };

            if let Some(real_user) = real_user {
                // effective user / puppetting
                let mut acting_user = if let Some(puppet_id) = headers.puppet_id {
                    let puppet = srv.users.get(puppet_id, None).await?;

                    if puppet.bot {
                        // bot owners can puppet their bots
                        // NOTE: not sure if this is useful in any way?
                        let app = state
                            .data()
                            .application_get(puppet.id.into_inner().into())
                            .await?;
                        if app.owner_id != real_user.id {
                            return Err(Error::ApiError(ApiError::from_code(
                                ErrorCode::NotBotOwner,
                            )));
                        }
                        puppet
                    } else {
                        // bridge bots can puppet their... uh... puppets...

                        if !real_user.bot {
                            return Err(Error::ApiError(ApiError::from_code(
                                ErrorCode::UserIsNotABot,
                            )));
                        }

                        let Some(p) = &puppet.puppet else {
                            return Err(Error::ApiError(ApiError::from_code(
                                ErrorCode::InvalidData,
                            ))); // TODO: more specific
                        };

                        if p.owner_id.into_inner() != *real_user.id {
                            return Err(Error::ApiError(ApiError::from_code(
                                ErrorCode::NotPuppetOwner,
                            )));
                        }

                        puppet
                    }
                } else {
                    real_user.clone()
                };

                // if the puppeteer is suspended, so is the puppet
                if acting_user.id != real_user.id && real_user.is_suspended() {
                    acting_user.suspended = real_user.suspended.clone();
                }

                if acting_user.suspended.is_none() {
                    // if the bot owner is suspended, so is the bot
                    if let Some(puppet) = &acting_user.puppet {
                        let bot_app_id = puppet.owner_id;
                        let bot_user = srv.users.get(bot_app_id.into_inner().into(), None).await?;
                        if bot_user.is_suspended() {
                            acting_user.suspended = bot_user.suspended.clone();
                        } else if bot_user.bot {
                            if let Ok(app) = state.data().application_get(bot_app_id).await {
                                let owner = srv.users.get(app.owner_id, None).await?;
                                if owner.is_suspended() {
                                    acting_user.suspended = owner.suspended.clone();
                                }
                            }
                        }
                    } else if acting_user.bot {
                        if let Ok(app) = state
                            .data()
                            .application_get(acting_user.id.into_inner().into())
                            .await
                        {
                            let owner = srv.users.get(app.owner_id, None).await?;
                            if owner.is_suspended() {
                                acting_user.suspended = owner.suspended.clone();
                            }
                        }
                    }
                }

                // scopes
                let scopes = if session.ty == SessionType::User {
                    Scopes(vec![Scope::Auth])
                } else if let Some(app_id) = session.app_id {
                    state
                        .data()
                        .connection_get(real_user.id, app_id)
                        .await
                        .map(|c| c.scopes)
                        .unwrap_or_default()
                } else {
                    Scopes::default()
                };

                if acting_user.id != real_user.id {
                    return Ok(Auth4 {
                        identity: Identity::Puppet {
                            puppet: acting_user,
                            puppeteer: real_user,
                            session,
                            scopes,
                        },
                        reason,
                        audit_txn_slot,
                    });
                } else {
                    return Ok(Auth4 {
                        identity: Identity::User {
                            user: acting_user,
                            session,
                            scopes,
                        },
                        reason,
                        audit_txn_slot,
                    });
                }
            } else {
                let scopes = if session.ty == SessionType::User {
                    Scopes(vec![Scope::Auth])
                } else {
                    Scopes::default()
                };

                return Ok(Auth4 {
                    identity: Identity::Guest { session, scopes },
                    reason,
                    audit_txn_slot,
                });
            }
        }

        // federation auth
        if let Some(FederationIdentity(origin)) =
            parts.extensions.get::<FederationIdentity>().cloned()
        {
            let puppet = if let Some(puppet_id) = headers.puppet_id {
                let user = srv.users.get(puppet_id, None).await?;
                if user.remote.as_ref().is_some_and(|r| r.hostname == origin) {
                    Some(user)
                } else {
                    return Err(Error::ApiError(ApiError::from_code(
                        ErrorCode::CannotManageRemoteUser,
                    )));
                }
            } else {
                None
            };

            return Ok(Auth4 {
                identity: Identity::Server {
                    hostname: origin,
                    puppet,
                },
                reason,
                audit_txn_slot,
            });
        }

        // public endpoints
        Ok(Auth4 {
            identity: Identity::Public,
            reason,
            audit_txn_slot,
        })
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth4 {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, s: &Arc<ServerState>) -> Result<Self> {
        Self::calculate(parts, s).await
    }
}
