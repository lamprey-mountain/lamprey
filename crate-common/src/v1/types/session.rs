use lamprey_macros::{Diff, record};
use std::{fmt, str::FromStr};

use crate::v1::types::{ApplicationId, util::Time};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use super::{UserId, ids::SessionId};

// TODO(#250): verify Hash here is timing safe?
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "utoipa",
    derive(utoipa::ToSchema),
    schema(examples("super_secret_session_token"))
)]
pub struct SessionToken(pub String);

#[record]
#[derive(PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,

    #[serde(flatten)]
    pub status: SessionStatus,

    /// a human readable name for this session
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub ty: SessionType,

    /// when this token will expire. only set for oauth auth tokens
    pub expires_at: Option<Time>,

    /// the oauth application this belongs to
    pub app_id: Option<ApplicationId>,

    /// session imprint metadata
    pub imprint: SessionImprint,

    /// when this session was logged in
    pub authorized_at: Option<Time>,

    /// when this session was logged out
    pub deauthorized_at: Option<Time>,

    /// if web push is enabled for this session
    #[cfg(any())]
    pub push_enabled: bool,
}

/// metadata that gets updated whenever the session is used
#[record]
#[derive(PartialEq, Eq)]
pub struct SessionImprint {
    /// the last time this session was used
    pub last_seen_at: Time,

    /// the ip address that accessed this session
    pub ip_addr: Option<String>,

    pub country_code: Option<String>,
    pub country_name: Option<String>,
    pub city_name: Option<String>,

    /// the user agent that was used while accessing this session
    pub user_agent: Option<String>,
}

/// minimal session persisted for audit log
#[record]
#[derive(PartialEq, Eq)]
pub struct SessionSummary {
    pub id: SessionId,
    pub name: Option<String>,
    pub app_id: Option<ApplicationId>,
    pub last_seen_at: Option<Time>,
    pub authorized_at: Time,
    pub deauthorized_at: Option<Time>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct SessionWithToken {
    #[serde(flatten)]
    pub session: Session,
    pub token: SessionToken,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct SessionCreate {
    #[schema(required = false, min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,
}

#[record]
#[derive(PartialEq, Eq, Diff)]
pub struct SessionPatch {
    #[schema(required = false)]
    #[serde(default, deserialize_with = "some_option")]
    pub name: Option<Option<String>>,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "status")]
pub enum SessionStatus {
    /// The session exists but can't do anything besides authenticate
    Unauthorized,

    /// The session exists and belongs to a user, but can't really do anything yet
    Bound { user_id: UserId },

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

impl SessionStatus {
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            SessionStatus::Unauthorized => None,
            SessionStatus::Bound { user_id } => Some(*user_id),
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

    pub fn ip_addr(&self) -> Option<&str> {
        self.imprint.ip_addr.as_deref()
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.imprint.user_agent.as_deref()
    }
}

// TODO: remove?
#[record]
#[derive(PartialEq, Eq)]
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

impl SessionImprint {
    /// create a new imprint with last_seen_at set to now
    pub fn new() -> Self {
        Self {
            last_seen_at: Time::now_utc(),
            ip_addr: None,
            country_code: None,
            country_name: None,
            city_name: None,
            user_agent: None,
        }
    }
}
