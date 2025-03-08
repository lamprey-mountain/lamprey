use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::util::{Diff, Time};

use super::{ids::SessionId, UserId};

// TODO(#250): verify Hash here is timing safe?
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
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
use crate::util::some_option;

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

        /// given ~5 minutes after auth to do stuff?
        expires_at: Time,
    },
}

// Granular session capability flags?
// enum SessionCapability {
//     Default,
//     Trusted,
//     Sudo,
// }

// the actual api design for usertrust pretty heavily depends on how antispam
// is implemented

// /// how trusted a user is
// enum UserTrust {
//     /// low permissions/trust, but intentionally
//     Guest,

//     /// likely spam
//     Suspicious,

//     /// don't know if its spam
//     Untrusted,

//     /// probably not spam
//     Trusted,
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
