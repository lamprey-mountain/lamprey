#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{Channel, Role, RoleId, RoleReorderItem, Room, RoomId, RoomMember};

/// something happened in a room
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DispatchRoom {
    pub room_id: RoomId,

    // /// the room sync sequence numb of this event, for offline sync
    // seq: RoomSeq,
    #[serde(flatten)]
    pub inner: DispatchRoomInner,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DispatchRoomInner {
    /// a room was created and/or you joined a room
    RoomCreate {
        room: Box<Room>,
        roles: Vec<Role>,
        channels: Vec<Channel>,
        threads: Vec<Channel>,

        /// your own room member
        room_member: Option<Box<RoomMember>>,
    },

    /// a room was updated
    RoomUpdate,

    /// a room was deleted, you left a room, or you were removed (kicked/banned) from a room
    RoomDelete,

    /// a role was created
    RoleCreate { role: Box<Role> },

    /// a role was updated
    RoleUpdate { role: Box<Role> },

    /// a role was deleted
    RoleDelete { role_id: RoleId },

    /// the role hierarchy was reordered
    RoleReorder { roles: Vec<RoleReorderItem> },
    // EmojiCreate {
    //     emoji: EmojiCustom,
    // },

    // EmojiUpdate {
    //     emoji: EmojiCustom,
    // },

    // EmojiDelete {
    //     emoji_id: EmojiId,
    //     room_id: RoomId,
    // },

    // AuditLogEntryCreate {
    //     entry: AuditLogEntry,
    // },

    // RoomMemberCreate {
    //     member: RoomMember,
    //     user: User,
    // },

    // RoomMemberUpdate {
    //     member: RoomMember,
    //     user: User,
    // },

    // RoomMemberDelete {
    //     room_id: RoomId,
    //     user_id: UserId,
    // },

    // BanCreate {
    //     room_id: RoomId,
    //     ban: RoomBan,
    // },

    // BanDelete {
    //     room_id: RoomId,
    //     user_id: UserId,
    // },

    // // TODO: split out AutomodManage with RoomManage?
    // /// an auto moderation rule was created. only sent to users with RoomManage.
    // AutomodRuleCreate {
    //     rule: AutomodRule,
    // },

    // /// an auto moderation rule was updated. only sent to users with RoomManage.
    // AutomodRuleUpdate {
    //     rule: AutomodRule,
    // },

    // /// an auto moderation rule was deleted. only sent to users with RoomManage.
    // AutomodRuleDelete {
    //     rule_id: AutomodRuleId,
    //     room_id: RoomId,
    // },

    // /// an auto moderation rule was executed. only sent to users with RoomManage.
    // AutomodRuleExecute {
    //     execution: AutomodRuleExecution,
    // },
}
