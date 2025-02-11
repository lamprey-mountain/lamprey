use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

/// Which permissions are granted to someone with Permission::Admin
pub const ADMIN_ROOM: &[Permission] = &[
    Permission::RoomManage,
    Permission::ThreadCreate,
    Permission::ThreadManage,
    Permission::ThreadDelete,
    Permission::MessageCreate,
    Permission::MessageFilesEmbeds,
    Permission::MessagePin,
    Permission::MessageDelete,
    Permission::MessageMassMention,
    Permission::MemberKick,
    Permission::MemberBan,
    Permission::MemberManage,
    Permission::InviteCreate,
    Permission::InviteManage,
    Permission::RoleManage,
    Permission::RoleApply,
];

// pub const ADMIN_THREAD: &[Permission] = &[
//     Permission::ThreadManage,
//     Permission::ThreadDelete,
//     Permission::MessageCreate,
//     Permission::MessageFilesEmbeds,
//     Permission::MessagePin,
//     Permission::MessageDelete,
//     Permission::MessageMassMention,
//     // Permission::MemberKick,
//     // Permission::MemberBan,
//     // Permission::MemberManage,
//     Permission::InviteCreate,
//     Permission::InviteManage,
// ];
