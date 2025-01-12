use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{ids::SessionId, UserId};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[schema(examples("super_secret_session_token"))]
pub struct SessionToken(String);

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub token: SessionToken,
    pub status: SessionStatus,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "session_status")]
pub enum SessionStatus {
	Unauthorized,
	Authorized,
	Sudo,
}

impl From<String> for SessionToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<SessionToken> for String {
    fn from(val: SessionToken) -> Self {
        val.0
    }
}
