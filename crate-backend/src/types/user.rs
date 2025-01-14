use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::UserId;

#[derive(
    Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type,
)]
pub struct User {
    pub id: UserId,
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
