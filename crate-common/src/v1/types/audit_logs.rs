use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::v1::types::{
    role::RoleReorderItem, util::Time, AuditLogEntryId, EmojiId, InviteCode, MessageId,
    MessageVerId, PermissionOverwriteType, RoleId, RoomId, SessionId, ThreadId, UserId,
};

// TODO: coalesce multiple events into one event, if possible
// eg. multiple FooUpdates from the same user
// or add bulk kick/ban audit log events and merge everything there
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogEntry {
    /// Unique id idenfitying this entry
    pub id: AuditLogEntryId,

    /// Room this happened in. Is user_id for user audit logs.
    pub room_id: RoomId,

    /// User who caused this entry to be created
    pub user_id: UserId,

    /// Session of the user who caused this, for user audit logs
    // TODO: set and save this field
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
#[serde(tag = "type", content = "metadata")]
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

    RoleReorder {
        roles: Vec<RoleReorderItem>,
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
        #[serde(rename = "type")]
        ty: PermissionOverwriteType,
        changes: Vec<AuditLogChange>,
    },

    ThreadOverwriteDelete {
        thread_id: ThreadId,
        overwrite_id: Uuid,
    },

    MemberKick {
        // TODO: remove (redundant)
        room_id: RoomId,
        user_id: UserId,
    },

    // TODO: rename to BanCreate
    MemberBan {
        // TODO: remove (redundant)
        room_id: RoomId,
        user_id: UserId,
    },

    // TODO: rename to BanDelete
    MemberUnban {
        // TODO: remove (redundant)
        room_id: RoomId,
        user_id: UserId,
    },

    MemberUpdate {
        // TODO: remove (redundant)
        room_id: RoomId,
        user_id: UserId,
        changes: Vec<AuditLogChange>,
    },

    MemberDisconnect {
        thread_id: ThreadId,
        user_id: UserId,
    },

    MemberMove {
        changes: Vec<AuditLogChange>,
        user_id: UserId,
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

    ThreadMemberAdd {
        thread_id: ThreadId,
        user_id: UserId,
    },

    ThreadMemberRemove {
        thread_id: ThreadId,
        user_id: UserId,
    },

    // // cant be logged because this isn't yet implemented
    // MessagePin,
    // MessageUnpin,
    // MessageRemove,
    // MessageRestore,
    UserUpdate {
        changes: Vec<AuditLogChange>,
    },

    UserSuspend {
        expires_at: Option<Time>,
        user_id: UserId,
    },

    UserUnsuspend {
        user_id: UserId,
    },

    // TODO: impl these events
    // FriendAdd,
    // FriendRemove,
    // BlockAdd,
    // BlockRemove,
    // SessionLogin, // SessionCreate doesnt make sense because when sessions are created they aren't linked to any users
    // SessionUpdate,
    // SessionDelete,
    // AuthUpdate,
    // EmailUpdate,

    // // for server audit log, which doesnt exist yet
    // ServerUpdate,
    // RoomDelete,
    UserRegistered {
        user_id: UserId,
    },

    AdminWhisper {
        user_id: UserId,
        changes: Vec<AuditLogChange>,
    },

    AdminBroadcast {
        changes: Vec<AuditLogChange>,
    },
}
