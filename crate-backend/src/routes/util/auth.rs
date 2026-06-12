use std::{sync::Arc, time::Duration};

use crate::{
    prelude::*, routes::util::headers::HeadersRequest, routes::util::FederationIdentity,
    ServerState,
};
use axum::extract::FromRequestParts;
use common::v1::types::{
    federation::Hostname,
    ids::{SERVER_TOKEN_SESSION_ID, SERVER_USER_ID},
    oauth::{Scope, Scopes},
    util::Time,
    Session, SessionImprint, SessionStatus, SessionToken, SessionType, User,
};
use http::request::Parts;
pub struct Auth4 {
    identity: Identity,
}

#[derive(Debug)]
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
    Puppet { puppet: User, puppeteer: User },

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

    /// attempt to get the session, if known
    pub fn session(&self) -> Option<&Session> {
        match &self.identity {
            Identity::User { session, .. } => Some(session),
            Identity::Guest { session, .. } => Some(session),
            _ => None,
        }
    }

    pub fn scopes(&self) -> Option<&Scopes> {
        match &self.identity {
            Identity::User { scopes, .. } => Some(scopes),
            Identity::Oauth { scopes, .. } => Some(scopes),
            Identity::Guest { scopes, .. } => Some(scopes),
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
        match &self.identity {
            Identity::User { session, .. } => match &session.status {
                SessionStatus::Sudo {
                    sudo_expires_at, ..
                } => {
                    if *sudo_expires_at < Time::now_utc() {
                        // TODO(err): use ErrorCode::SudoSessionExpired
                        Err(Error::BadStatic("sudo session expired"))
                    } else {
                        Ok(())
                    }
                }
                // TODO(err): use ErrorCode::SudoRequired
                _ => Err(Error::BadStatic("needs sudo")),
            },
            // servers are implicitly sudo
            Identity::Server { .. } => Ok(()),
            _ => Err(Error::MissingAuth),
        }
    }
}

impl Auth4 {
    pub async fn calculate(parts: &Parts, state: &ServerState) -> Result<Self> {
        let srv = state.services();
        let headers = HeadersRequest::from_parts(parts);

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

            let user = if let Some(user_id) = session.user_id() {
                Some(srv.users.get(user_id, None).await?)
            } else {
                None
            };

            let scopes = if session.ty == SessionType::User {
                Scopes(vec![Scope::Auth])
            } else {
                Scopes::default()
            };

            if let Some(user) = user {
                return Ok(Auth4 {
                    identity: Identity::User {
                        user,
                        session,
                        scopes,
                    },
                });
            } else {
                return Ok(Auth4 {
                    identity: Identity::Guest { session, scopes },
                });
            }
        }

        // federation auth
        if let Some(FederationIdentity(origin)) =
            parts.extensions.get::<FederationIdentity>().cloned()
        {
            return Ok(Auth4 {
                identity: Identity::Server {
                    hostname: origin,
                    puppet: None, // TODO: puppet support
                },
            });
        }

        // public endpoints
        Ok(Auth4 {
            identity: Identity::Public,
        })
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth4 {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, s: &Arc<ServerState>) -> Result<Self> {
        Self::calculate(parts, s).await
    }
}
