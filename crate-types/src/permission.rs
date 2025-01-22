use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

    // unsure about these or how they interact with other perms
    // ThreadForward,
    // MessageMove,
    View,
    MessageEdit,
}
