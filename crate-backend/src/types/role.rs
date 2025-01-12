use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{RoleId, RoomId};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type)]
pub struct Role {
    pub id: RoleId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "permission")]
pub enum Permission {
    Admin,
    RoomManage,
    ThreadCreate,
    ThreadManage,
    ThreadDelete,
    MessageCreate,
    MessageFilesEmbeds,
    MessagePin,
    MessageDelete,
    MessageMassMention,
    MemberKick,
    MemberBan,
    MemberManage,
    InviteCreate,
    InviteManage,
    RoleManage,
    RoleApply,

    View,
    MessageEdit,
}
