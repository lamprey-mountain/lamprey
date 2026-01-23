use std::{fmt, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    util::{some_option, Diff, Time},
    ApplicationId,
};

use super::{ids::SessionId, UserId};

// TODO(#250): verify Hash here is timing safe?
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "utoipa",
    derive(ToSchema),
    schema(examples("super_secret_session_token"))
)]
pub struct SessionToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Session {
    pub id: SessionId,

    #[serde(flatten)]
    pub status: SessionStatus,

    /// a human readable name for this session
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub ty: SessionType,

    /// when this token will expire. only set for oauth auth tokens
    pub expires_at: Option<Time>,

    /// the oauth application this belongs to
    pub app_id: Option<ApplicationId>,

    /// the last time this session was used
    pub last_seen_at: Time,
    pub ip_addr: Option<String>,
    pub user_agent: Option<String>,

    /// when this session was logged in
    pub authorized_at: Option<Time>,

    /// when this session was logged out
    pub deauthorized_at: Option<Time>,
}

/// minimal session persisted for audit log
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionSummary {
    pub id: SessionId,
    pub name: Option<String>,
    pub app_id: Option<ApplicationId>,
    pub last_seen_at: Option<Time>,
    pub authorized_at: Time,
    pub deauthorized_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SessionWithToken {
    #[serde(flatten)]
    pub session: Session,
    pub token: SessionToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SessionCreate {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SessionPatch {
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    #[serde(default, deserialize_with = "some_option")]
    pub name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "status")]
pub enum SessionStatus {
    /// The session exists but can't do anything besides authenticate
    Unauthorized,

    /// The session exists and can do non-critical actions
    Authorized { user_id: UserId },

    /// The session exists and can do administrative actions
    Sudo {
        user_id: UserId,
        sudo_expires_at: Time,
    },
}

impl From<String> for SessionToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Diff<Session> for SessionPatch {
    fn changes(&self, other: &Session) -> bool {
        self.name.changes(&other.name)
    }
}

impl SessionStatus {
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            SessionStatus::Unauthorized => None,
            SessionStatus::Authorized { user_id } => Some(*user_id),
            SessionStatus::Sudo { user_id, .. } => Some(*user_id),
        }
    }
}

impl Session {
    pub fn can_see(&self, other: &Self) -> bool {
        match (self.status.user_id(), other.status.user_id()) {
            (Some(a), Some(b)) if a == b => true,
            _ if self.id == other.id => true,
            _ => false,
        }
    }

    pub fn user_id(&self) -> Option<UserId> {
        self.status.user_id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SessionType {
    /// an user token
    // NOTE: i might remove this and switch to purely oauth
    User,

    /// a session created via oauth2
    Access,
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SessionType::User => "User",
            SessionType::Access => "Access",
        };
        f.write_str(s)
    }
}

impl FromStr for SessionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "User" => Ok(SessionType::User),
            "Access" => Ok(SessionType::Access),
            _ => Err(()),
        }
    }
}
