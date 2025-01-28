use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::util::Diff;

use super::{ids::SessionId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "utoipa",
    derive(ToSchema),
    schema(examples("super_secret_session_token"))
)]
pub struct SessionToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Session {
    pub id: SessionId,
    #[serde(flatten)]
    pub status: SessionStatus,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionWithToken {
    #[serde(flatten)]
    pub session: Session,
    pub token: SessionToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionCreate {
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionPatch {
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub name: Option<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "status")]
pub enum SessionStatus {
    /// The session exists but can't do anything besides authenticate
    Unauthorized,

    /// The session exists and can do non-critical actions
    Authorized { user_id: UserId },

    // /// The session is probably not a bot (ie. solved a captcha)
    // Trusted,
    /// The session exists and can do administrative actions
    Sudo { user_id: UserId },
}

// Granular session capability flags?
// enum SessionCapability {
//     Default,
//     Trusted,
//     Sudo,
// }

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
    pub fn user_id(self) -> Option<UserId> {
        match self {
            SessionStatus::Unauthorized => None,
            SessionStatus::Authorized { user_id } => Some(user_id),
            SessionStatus::Sudo { user_id } => Some(user_id),
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
