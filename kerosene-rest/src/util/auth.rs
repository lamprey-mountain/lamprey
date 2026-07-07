use crate::prelude::*;
use common::{
    v1::types::{
        Session, SessionStatus, User,
        error::{ApiError, ErrorCode},
        federation::Hostname,
        oauth::{Scope, Scopes},
        util::Time,
    },
    v2::types::UserId,
};

pub struct Auth {
    identity: Identity,
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

impl Auth {
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
        self.user()
            .ok_or_else(|| ApiError::from(ErrorCode::MissingAuth).into())
    }

    pub fn ensure_session(&self) -> Result<&Session> {
        self.session()
            .ok_or_else(|| ApiError::from(ErrorCode::MissingAuth).into())
    }

    pub fn ensure_origin(&self) -> Result<&Hostname> {
        self.origin()
            .ok_or_else(|| ApiError::from(ErrorCode::MissingAuth).into())
    }

    pub fn ensure_scopes(&self, scopes: &[Scope]) -> Result<()> {
        let self_scopes = self
            .scopes()
            .ok_or_else(|| ApiError::from(ErrorCode::MissingAuth))?;
        if let Err(_) = self_scopes.ensure_all(scopes) {
            // NOTE: i don't actually know if these are *all* the required scopes or not (what if ensure_scopes gets called twice?), but its probably good enough for now
            let mut api_error = ApiError::from(ErrorCode::MissingScopes);
            api_error.required_scopes = scopes.to_vec();
            return Err(api_error.into());
        }
        Ok(())
    }

    pub fn ensure_sudo(&self) -> Result<()> {
        // servers are always sudo
        if let Identity::Server { .. } = &self.identity {
            return Ok(());
        }

        let session = self
            .session()
            .ok_or_else(|| ApiError::from(ErrorCode::MissingAuth))?;
        match &session.status {
            SessionStatus::Sudo {
                sudo_expires_at, ..
            } => {
                if sudo_expires_at < &Time::now_utc() {
                    Err(ApiError::from_code(ErrorCode::SudoSessionExpired).into())
                } else {
                    Ok(())
                }
            }
            _ => Err(ApiError::from_code(ErrorCode::SudoRequired).into()),
        }
    }
}

// TODO: copy crate-backend/src/routes/util/auth.rs here
