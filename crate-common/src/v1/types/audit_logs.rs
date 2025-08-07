use crate::v1::types::{AuditLogId, MessageSync, RoomId, UserId};

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO(#239): redesign audit log schema, since recursion
// also causes some issues when trying to load old data, need to add migrations or #[serde(default)] attrs
// TODO: rename to AuditLogEntry and AuditLogEntryId

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLog {
    /// Unique id idenfitying this entry
    pub id: AuditLogId,

    /// Room this happened in
    pub room_id: RoomId,

    /// User who caused this entry to be created
    pub user_id: UserId,

    /// User supplied reason why this happened
    pub reason: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(no_recursion))]
    /// Generated sync payload (sent in websocket)
    pub payload: Box<Value>,

    #[cfg_attr(feature = "utoipa", schema(no_recursion))]
    /// The previous payload, or None if this resource is newly created
    pub payload_prev: Option<Box<Value>>,
}

mod next {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use utoipa::ToSchema;

    use crate::v1::types::{
        AuditLogId, EmojiId, InviteCode, MessageId, MessageVerId, RoleId, RoomId, SessionId,
        ThreadId, UserId,
    };

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    pub struct AuditLogEntry {
        /// Unique id idenfitying this entry
        pub id: AuditLogId,

        /// Room this happened in
        pub room_id: RoomId,

        /// User who caused this entry to be created
        pub user_id: UserId,

        /// Session of the user who caused this
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
        pub key: Value,
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

        // below aren't sync events
        ThreadOverwriteSet,
        ThreadOverwriteDelete,
        MemberKick,
        MemberBan,
        MemberUnban,
        MemberUpdate,
        RoleApply,
        RoleUnapply,
        MessagePin,
        MessageUnpin,
        BotAdd,
        MessageRemove,
        MessageRestore,

        // user events
        UserUpdate,
        FriendAdd,
        FriendRemove,
        BlockAdd,
        BlockRemove,
    }
}
