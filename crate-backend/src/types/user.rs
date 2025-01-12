use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::UserId;

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type)]
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
