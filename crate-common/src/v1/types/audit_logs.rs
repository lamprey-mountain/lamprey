#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_json::Value;
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::Scope, email::EmailAddr, reaction::ReactionKeyParam, role::RoleReorderItem,
    util::Time, ApplicationId, AuditLogEntryId, CalendarEventId, ChannelId, ChannelReorderItem,
    ChannelType, EmojiId, InviteCode, MessageId, MessageVerId, PermissionOverwriteType, RoleId,
    RoomId, SessionId, UserId, WebhookId,
};

// TODO: coalesce multiple events into one event, if possible
// eg. multiple FooUpdates from the same user
// or add bulk kick/ban audit log events and merge everything there
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    ChannelCreate {
        channel_id: ChannelId,
        channel_type: ChannelType,
        changes: Vec<AuditLogChange>,
    },

    ChannelUpdate {
        channel_id: ChannelId,
        channel_type: ChannelType,
        changes: Vec<AuditLogChange>,
    },

    MessageDelete {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    MessageVersionDelete {
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    MessageDeleteBulk {
        channel_id: ChannelId,
        message_ids: Vec<MessageId>,
    },

    MessageRemove {
        channel_id: ChannelId,
        message_ids: Vec<MessageId>,
    },

    MessageRestore {
        channel_id: ChannelId,
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
    #[deprecated = "renamed to ReactionDeleteAll"]
    ReactionPurge {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    /// remove all reactions from a message
    ReactionDeleteAll {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    /// remove all reactions of an emoji from a message
    ReactionDeleteKey {
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    },

    /// remove a reactions from a specific user on a message
    ReactionDeleteUser {
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
        user_id: UserId,
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

    PermissionOverwriteSet {
        channel_id: ChannelId,
        overwrite_id: Uuid,
        #[serde(rename = "type")]
        ty: PermissionOverwriteType,
        changes: Vec<AuditLogChange>,
    },

    PermissionOverwriteDelete {
        channel_id: ChannelId,
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

    MemberPrune {
        /// number of pruned users
        pruned: u64,
    },

    MemberUpdate {
        // TODO: remove (redundant)
        room_id: RoomId,
        user_id: UserId,
        changes: Vec<AuditLogChange>,
    },

    MemberDisconnect {
        channel_id: ChannelId,
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
        thread_id: ChannelId,
        user_id: UserId,
    },

    ThreadMemberRemove {
        thread_id: ChannelId,
        user_id: UserId,
    },

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

    /// friend request sent to another user
    FriendRequest {
        user_id: UserId,
    },

    /// friend request from another user accepted
    FriendAccept {
        user_id: UserId,
    },

    FriendDelete {
        user_id: UserId,
    },

    BlockCreate {
        user_id: UserId,
    },

    BlockDelete {
        user_id: UserId,
    },

    IgnoreAdd {
        user_id: UserId,
    },

    IgnoreRemove {
        user_id: UserId,
    },

    SessionLogin {
        user_id: UserId,
        session_id: SessionId,
    },

    SessionUpdate {
        session_id: SessionId,
        changes: Vec<AuditLogChange>,
    },

    SessionDelete {
        session_id: SessionId,
    },

    SessionDeleteAll,

    /// auth state changed
    AuthUpdate {
        changes: Vec<AuditLogChange>,
    },

    /// user entered sudo mode
    AuthSudo {
        session_id: SessionId,
    },

    ApplicationCreate {
        application_id: ApplicationId,
        changes: Vec<AuditLogChange>,
    },

    ApplicationUpdate {
        application_id: ApplicationId,
        changes: Vec<AuditLogChange>,
    },

    ApplicationDelete {
        application_id: ApplicationId,
    },

    EmailCreate {
        email: EmailAddr,
        changes: Vec<AuditLogChange>,
    },

    EmailUpdate {
        email: EmailAddr,
        changes: Vec<AuditLogChange>,
    },

    EmailDelete {
        email: EmailAddr,
    },

    ConnectionCreate {
        application_id: ApplicationId,
        scopes: Vec<Scope>,
    },

    ConnectionDelete {
        application_id: ApplicationId,
    },

    UserRegistered {
        user_id: UserId,
    },

    UserDelete {
        user_id: UserId,
    },

    UserUndelete {
        user_id: UserId,
    },

    AdminWhisper {
        user_id: UserId,
        changes: Vec<AuditLogChange>,
    },

    AdminBroadcast {
        changes: Vec<AuditLogChange>,
    },

    RoomDelete {
        room_id: RoomId,
    },

    RoomUndelete {
        room_id: RoomId,
    },

    RoomQuarantine {
        room_id: RoomId,
    },

    RoomUnquarantine {
        room_id: RoomId,
    },

    MessagePin {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    MessageUnpin {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    MessagePinReorder {
        channel_id: ChannelId,
    },

    ChannelReorder {
        channels: Vec<ChannelReorderItem>,
    },

    CalendarEventCreate {
        changes: Vec<AuditLogChange>,
    },

    CalendarEventUpdate {
        changes: Vec<AuditLogChange>,
    },

    CalendarEventDelete {
        title: String,
        event_id: CalendarEventId,
    },

    WebhookCreate {
        webhook_id: WebhookId,
        changes: Vec<AuditLogChange>,
    },

    WebhookUpdate {
        webhook_id: WebhookId,
        changes: Vec<AuditLogChange>,
    },

    WebhookDelete {
        webhook_id: WebhookId,
    },

    RatelimitUpdate {
        channel_id: ChannelId,
        user_id: UserId,
        slowmode_thread_expire_at: Option<Time>,
        slowmode_message_expire_at: Option<Time>,
    },
    // // TODO: for server audit log; log when routes for these are implemented
    // ServerUpdate,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct AuditLogFilter {
    /// only return audit log entries from these users
    #[serde(default)]
    pub user_id: Vec<UserId>,

    /// only return audit log entries with these types
    #[serde(default, rename = "type")]
    pub ty: Vec<String>,
}
