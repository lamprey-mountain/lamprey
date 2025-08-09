use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::v1::types::{
    AuditLogEntryId, EmojiId, InviteCode, MessageId, MessageVerId, Permission,
    PermissionOverwriteType, RoleId, RoomId, SessionId, ThreadId, UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogEntry {
    /// Unique id idenfitying this entry
    pub id: AuditLogEntryId,

    /// Room this happened in
    pub room_id: RoomId,

    /// User who caused this entry to be created
    pub user_id: UserId,

    /// Session of the user who caused this
    // will be for user audit logs
    pub session_id: Option<SessionId>,

    /// User supplied reason why this happened
    pub reason: Option<String>,

    #[serde(flatten)]
    pub ty: AuditLogEntryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogChange {
    pub new: Value,
    pub old: Value,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum AuditLogEntryType {
    RoomCreate {
        changes: Vec<AuditLogChange>,
    },

    RoomUpdate {
        changes: Vec<AuditLogChange>,
    },

    ThreadCreate {
        thread_id: ThreadId,
        changes: Vec<AuditLogChange>,
    },

    ThreadUpdate {
        thread_id: ThreadId,
        changes: Vec<AuditLogChange>,
    },

    MessageDelete {
        thread_id: ThreadId,
        message_id: MessageId,
    },

    MessageVersionDelete {
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    MessageDeleteBulk {
        thread_id: ThreadId,
        message_ids: Vec<MessageId>,
    },

    RoleCreate {
        changes: Vec<AuditLogChange>,
    },

    RoleUpdate {
        changes: Vec<AuditLogChange>,
    },

    RoleDelete {
        role_id: RoleId,
    },

    InviteCreate {
        changes: Vec<AuditLogChange>,
    },

    InviteUpdate {
        changes: Vec<AuditLogChange>,
    },

    InviteDelete {
        code: InviteCode,
    },

    /// remove all reactions
    ReactionPurge {
        thread_id: ThreadId,
        message_id: MessageId,
    },

    EmojiCreate {
        changes: Vec<AuditLogChange>,
    },

    EmojiUpdate {
        changes: Vec<AuditLogChange>,
    },

    EmojiDelete {
        emoji_id: EmojiId,
    },

    ThreadOverwriteSet {
        thread_id: ThreadId,
        overwrite_id: Uuid,
        ty: PermissionOverwriteType,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    },
    ThreadOverwriteDelete {
        thread_id: ThreadId,
        overwrite_id: Uuid,
    },
    MemberKick {
        room_id: RoomId,
        user_id: UserId,
    },
    MemberBan {
        room_id: RoomId,
        user_id: UserId,
    },
    MemberUnban {
        room_id: RoomId,
        user_id: UserId,
    },
    MemberUpdate {
        room_id: RoomId,
        user_id: UserId,
        changes: Vec<AuditLogChange>,
    },
    RoleApply {
        user_id: UserId,
        role_id: RoleId,
    },
    RoleUnapply {
        user_id: UserId,
        role_id: RoleId,
    },
    BotAdd {
        // TODO: rename to application_id?
        bot_id: UserId,
    },
    // // cant be logged because this isn't yet implemented
    // MessagePin,
    // MessageUnpin,
    // MessageRemove,
    // MessageRestore,

    // // for user audit log, which doesn't exist yet
    // UserUpdate,
    // FriendAdd,
    // FriendRemove,
    // BlockAdd,
    // BlockRemove,
}
