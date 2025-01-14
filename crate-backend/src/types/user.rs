use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{UserId, UserVerId};

#[derive(
    Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type,
)]
pub struct User {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    // email: Option<String>,
    // avatar: Option<String>,
    pub is_bot: bool,
    pub is_alias: bool,
    pub is_system: bool,
}

pub struct UserRow {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<uuid::Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    // email: Option<String>,
    // avatar: Option<String>,
    pub is_bot: bool,
    pub is_alias: bool,
    pub is_system: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type,
)]
pub struct UserCreate {
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub is_bot: bool,
    pub is_alias: bool,
    pub is_system: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type,
)]
pub struct UserCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub is_bot: bool,
    pub is_alias: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type,
)]
pub struct UserPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<Option<String>>,
    pub is_bot: Option<bool>,
    pub is_alias: Option<bool>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: row.id,
            version_id: row.version_id,
            parent_id: row.parent_id.map(UserId),
            name: row.name,
            description: row.description,
            status: row.status,
            is_bot: row.is_bot,
            is_alias: row.is_alias,
            is_system: row.is_system,
        }
    }
}
