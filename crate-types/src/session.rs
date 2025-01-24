use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{ids::SessionId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "utoipa",
    derive(ToSchema),
    schema(examples("super_secret_session_token"))
)]
pub struct SessionToken(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Session {
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub id: SessionId,
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub user_id: UserId,
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub token: SessionToken,
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub status: SessionStatus,
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionCreate {
    #[cfg_attr(feature = "utoipa", schema(write_only))]
    pub user_id: UserId,

    #[cfg_attr(feature = "utoipa", schema(write_only, required = false))]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionPatch {
    #[cfg_attr(feature = "utoipa", schema(write_only, required = false))]
    pub name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SessionStatus {
    /// The session exists but can't do anything besides authenticate
    Unauthorized,
    
    /// The session exists and can do non-critical actions
    Authorized,
    
    // /// The session is probably not a bot (ie. solved a captcha)
    // Trusted,
    
    /// The session exists and can do administrative actions
    Sudo,
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

impl SessionPatch {
    pub fn wont_change(&self, target: &Session) -> bool {
        self.name.as_ref().is_none_or(|n| n == &target.name)
    }
}
