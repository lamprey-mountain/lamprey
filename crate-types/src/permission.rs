use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{util::deserialize_sorted_permissions, RoleId, UserId};

// TODO: redo permissions

/// a permission that lets a user do something
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Permission {
    // this is a major footgun. i'd like to remove it, but theres legit purposes for it right now...
    Admin,

    RoomManage,
    // RoomEdit,
    // BotsAdd,
    // BotsManage,
    // EmojiManage,
    // EmojiUseExternal,
    ThreadCreate,
    // ThreadCreateChat,
    // ThreadCreateForumLinear,
    // ThreadCreateForumTree,
    // ThreadCreateVoice,
    ThreadManage,
    // ThreadPublish, // announcements?
    // ThreadEdit,
    // ThreadArchive,
    // ThreadPin,
    ThreadDelete,
    MessageCreate,
    MessageFilesEmbeds,
    // // might want to split that into two perms
    // MessageMedia,
    // MessageEmbeds,
    // // permission to revent automatically generating url previews?
    // // idk if this is any useful though
    // MessageAutoEmbed,
    MessagePin,
    MessageDelete,
    MessageMassMention,
    // MessageReactAdd,
    // MessageReactExisting,
    MemberKick,
    MemberBan,
    MemberManage,
    // ProfileOverrideRoom,
    // ProfileOverrideThread,
    InviteCreate,
    InviteManage,
    RoleManage,
    RoleApply,
    // AuditLogView,
    // TagManage,
    // TagApply,

    // unsure about these or how they interact with other perms
    // ThreadForward,
    // MessageMove,

    // voice permissions (for the future)
    // VoiceConnect,
    // VoiceSpeak,
    // VoiceVideo,
    // VoiceMute,
    // VoiceDeafen,
    // VoiceDisconnect,
    // VoiceMove,

    // these are automatically granted/calculated and cannot be manually assigned
    View,
    MessageEdit,
    // user level permissions
    // UserStatus, // set status

    // server level permissions
    // ServerUserList,
    // ServerUserManage,
    // ServerRoomList, // rooms, threads, everything in them
    // ServerRoomManage,
    // ServerMetrics, // auth for otel/prometheus?
    // ServerAdmin, // "root user" permission. probably shouldn't implement this.
}

/// Which permissions are granted to someone with Permission::Admin in a room
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

/// Which permissions are granted to someone with Permission::Admin in a thread
pub const ADMIN_THREAD: &[Permission] = &[
    Permission::ThreadManage,
    Permission::ThreadDelete,
    Permission::MessageCreate,
    Permission::MessageFilesEmbeds,
    Permission::MessagePin,
    Permission::MessageDelete,
    Permission::MessageMassMention,
    Permission::InviteCreate,
    Permission::InviteManage,
    // member permissions are only for this thread
    Permission::MemberKick,
    Permission::MemberBan,
    Permission::MemberManage,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverrides {
    #[serde(flatten)]
    inner: Vec<PermissionOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum PermissionOverridable {
    /// permission overrides for a role
    Role(RoleId),

    /// permission overrides for a user
    User(UserId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PermissionOverride {
    #[serde(flatten)]
    pub target: PermissionOverridable,

    /// extra permissions allowed here
    #[serde(deserialize_with = "deserialize_sorted_permissions")]
    pub allow: Vec<Permission>,

    /// permissions denied here
    #[serde(deserialize_with = "deserialize_sorted_permissions")]
    pub deny: Vec<Permission>,
}
