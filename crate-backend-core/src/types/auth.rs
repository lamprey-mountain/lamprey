use common::{
    v1::types::{
        AuditLogEntryStatus, AuditLogEntryType, Session, SessionStatus, User,
        error::{ApiError, ApiResult, ErrorCode},
        federation::Hostname,
        oauth::{Scope, Scopes},
        util::Time,
    },
    v2::types::{RoomId, UserId},
};

use crate::prelude::*;

/// the identity of someone making a request
// TODO: use Arc<Session>
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

impl Identity {
    /// get the acting user
    ///
    /// for puppet/server, returns the puppeted user
    pub fn user(&self) -> Option<&User> {
        match self {
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
        match self {
            Identity::User { session, .. } => Some(session),
            Identity::Guest { session, .. } => Some(session),
            Identity::Puppet { session, .. } => Some(session),
            _ => None,
        }
    }

    pub fn scopes(&self) -> Option<&Scopes> {
        match self {
            Identity::User { scopes, .. } => Some(scopes),
            Identity::Oauth { scopes, .. } => Some(scopes),
            Identity::Guest { scopes, .. } => Some(scopes),
            Identity::Puppet { scopes, .. } => Some(scopes),
            _ => None,
        }
    }

    pub fn origin(&self) -> Option<&Hostname> {
        match self {
            Identity::Server { hostname, .. } => Some(hostname),
            _ => None,
        }
    }

    /// like `self.user()` but returns an error instead of `None`
    pub fn ensure_user(&self) -> ApiResult<&User> {
        self.user().ok_or_else(|| {
            ApiError::with_message(ErrorCode::MissingAuth, "endpoint requires a user".into())
        })
    }

    pub fn ensure_session(&self) -> ApiResult<&Session> {
        self.session().ok_or_else(|| {
            ApiError::with_message(ErrorCode::MissingAuth, "endpoint requires a session".into())
        })
    }

    pub fn ensure_origin(&self) -> ApiResult<&Hostname> {
        self.origin().ok_or_else(|| {
            ApiError::with_message(ErrorCode::MissingAuth, "endpoint requires a origin".into())
        })
    }

    pub fn ensure_scopes(&self, scopes: &[Scope]) -> ApiResult<()> {
        let self_scopes = self.scopes().ok_or_else(|| ApiError {
            required_scopes: scopes.to_vec(),
            ..ApiError::from_code(ErrorCode::MissingScopes)
        })?;
        self_scopes.ensure_all(scopes)
    }

    pub fn ensure_sudo(&self) -> ApiResult<()> {
        // servers are always sudo
        if let Identity::Server { .. } = &self {
            return Ok(());
        }

        let session = self.ensure_session()?;
        match &session.status {
            SessionStatus::Sudo {
                sudo_expires_at, ..
            } => {
                if *sudo_expires_at < Time::now_utc() {
                    Err(ApiError::from_code(ErrorCode::SudoSessionExpired))
                } else {
                    Ok(())
                }
            }
            _ => Err(ApiError::from_code(ErrorCode::SudoRequired)),
        }
    }
}

/// authentication and authorization state for a request
pub trait Auth5: Send {
    fn identity(&self) -> &Identity;

    /// set the room id for this request, for audit logging
    fn set_room_id(&mut self, room_id: RoomId);

    /// create an audit log entry for this request
    fn al_push(&mut self, ty: AuditLogEntryType);

    /// set the status to use for audit log entries
    fn al_status(&mut self, status: AuditLogEntryStatus);
}

pub trait Auth5Ext: Auth5 {
    fn ensure_user(&self) -> ApiResult<&User> {
        self.identity().ensure_user()
    }

    fn ensure_session(&self) -> ApiResult<&Session> {
        self.identity().ensure_session()
    }

    fn ensure_origin(&self) -> ApiResult<&Hostname> {
        self.identity().ensure_origin()
    }

    fn ensure_scopes(&self, scopes: &[Scope]) -> ApiResult<()> {
        self.identity().ensure_scopes(scopes)
    }

    fn ensure_sudo(&self) -> ApiResult<()> {
        self.identity().ensure_sudo()
    }
}

impl<T: Auth5> Auth5Ext for T {}
