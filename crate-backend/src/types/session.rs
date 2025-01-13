use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{ids::SessionId, UserId};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[schema(examples("super_secret_session_token"))]
pub struct SessionToken(String);

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
pub struct Session {
    #[schema(read_only)]
    pub id: SessionId,
    #[schema(read_only)]
    pub user_id: UserId,
    #[schema(read_only)]
    pub token: SessionToken,
    #[schema(read_only)]
    pub status: SessionStatus,
    #[schema(read_only)]
    pub name: Option<String>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
pub struct SessionCreate {
    #[schema(write_only)]
    pub user_id: UserId,

    #[schema(write_only, required = false)]
    pub name: Option<String>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
pub struct SessionPatch {
    #[schema(write_only, required = false)]
    pub name: Option<Option<String>>,
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
