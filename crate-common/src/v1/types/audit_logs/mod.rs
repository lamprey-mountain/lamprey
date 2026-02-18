use std::time::Duration;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_json::Value;
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::Scopes, email::EmailAddr, reaction::ReactionKeyParam, role::RoleReorderItem,
    util::Time, webhook::Webhook, ApplicationId, AuditLogEntryId, AutomodRuleId, CalendarEventId,
    Channel, ChannelId, ChannelReorderItem, ChannelType, EmojiId, HarvestId, InviteCode, MessageId,
    MessageVerId, PermissionOverwriteType, RoleId, RoomId, RoomMember, SessionId, User, UserId,
    WebhookId,
};

pub mod resolve;

// FIXME(#981): bridge events should be logged as the bridge, not puppet
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogEntry {
    /// Unique id idenfitying this entry
    pub id: AuditLogEntryId,

    /// Room this happened in. Is user_id for user audit logs.
    // TODO: rename to context_id?
    pub room_id: RoomId,

    /// User who caused this entry to be created
    pub user_id: UserId,

    /// Session of the user who caused this, for user audit logs
    pub session_id: Option<SessionId>,

    /// User supplied reason why this happened
    pub reason: Option<String>,

    /// type and metadata for this audit log entry
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: AuditLogEntryType,

    /// the status of the request
    pub status: AuditLogEntryStatus,

    /// when the request started
    pub started_at: Time,

    /// when the request ended
    pub ended_at: Time,

    /// the ip address that this request came from
    ///
    /// will be None if you do not have permission to see sensitive request metadata or if it is not known
    pub ip_addr: Option<String>,

    /// the user agent that this request came from
    ///
    /// will be None if you do not have permission to see sensitive request metadata or if it is not known
    pub user_agent: Option<String>,

    /// if this was done via an oauth app, this is the application id responsible for the request
    pub application_id: Option<ApplicationId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogChange {
    pub new: Value,
    pub old: Value,
    pub key: String,
}

// NOTE: maybe i want to also have Channel{Remove,Restore}?
// NOTE: maybe i want to also have Thread{Create,Update,Etc}?
// NOTE: maybe i should hoist changes to the top level...?
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "metadata"))]
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    #[deprecated = "this has been split into PermissionOverwriteCreate and PermissionOverwriteUpdate"]
    PermissionOverwriteSet {
        channel_id: ChannelId,
        overwrite_id: Uuid,
        #[cfg_attr(feature = "serde", serde(rename = "type"))]
        ty: PermissionOverwriteType,
        changes: Vec<AuditLogChange>,
    },

    PermissionOverwriteCreate {
        channel_id: ChannelId,
        overwrite_id: Uuid,
        #[cfg_attr(feature = "serde", serde(rename = "type"))]
        ty: PermissionOverwriteType,
        changes: Vec<AuditLogChange>,
    },

    PermissionOverwriteUpdate {
        channel_id: ChannelId,
        overwrite_id: Uuid,
        #[cfg_attr(feature = "serde", serde(rename = "type"))]
        ty: PermissionOverwriteType,
        changes: Vec<AuditLogChange>,
    },

    PermissionOverwriteDelete {
        channel_id: ChannelId,
        overwrite_id: Uuid,
        #[cfg_attr(feature = "serde", serde(rename = "type"))]
        ty: PermissionOverwriteType,
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

    MemberDisconnectAll {
        channel_id: ChannelId,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    ConnectionCreate {
        application_id: ApplicationId,
        scopes: Scopes,
    },

    ConnectionDelete {
        application_id: ApplicationId,
    },

    UserRegistered {
        user_id: UserId,
    },

    UserDelete {
        user_id: UserId,

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    UserUndelete {
        user_id: UserId,
    },

    HarvestCreate {
        harvest_id: HarvestId,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
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
        event_id: CalendarEventId,

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    CalendarOverwriteCreate {
        event_id: CalendarEventId,
        seq: u64,
        changes: Vec<AuditLogChange>,
    },

    CalendarOverwriteUpdate {
        event_id: CalendarEventId,
        seq: u64,
        changes: Vec<AuditLogChange>,
    },

    CalendarOverwriteDelete {
        event_id: CalendarEventId,
        seq: u64,

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    /// someone else's rsvp was deleted
    ///
    /// not emitted when someone deletes their own rsvp
    CalendarRsvpDelete {
        event_id: CalendarEventId,

        /// populated if this is for an overwrite
        seq: Option<u64>,

        user_id: UserId,
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

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    RatelimitUpdate {
        channel_id: ChannelId,
        user_id: UserId,
        slowmode_thread_expire_at: Option<Time>,
        slowmode_message_expire_at: Option<Time>,
    },

    RatelimitDelete {
        channel_id: ChannelId,
        user_id: UserId,
    },

    RatelimitDeleteAll {
        channel_id: ChannelId,
    },

    AutomodRuleCreate {
        rule_id: AutomodRuleId,
        changes: Vec<AuditLogChange>,
    },

    AutomodRuleUpdate {
        rule_id: AutomodRuleId,
        changes: Vec<AuditLogChange>,
    },

    AutomodRuleDelete {
        rule_id: AutomodRuleId,

        #[cfg_attr(feature = "serde", serde(default))]
        changes: Vec<AuditLogChange>,
    },

    ChannelReindex {
        channel_id: ChannelId,
    },

    ServerUpdate {
        hostname: String,
        changes: Vec<AuditLogChange>,
    },
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct AuditLogFilter {
    /// only return audit log entries from these users
    #[cfg_attr(feature = "serde", serde(default))]
    pub user_id: Vec<UserId>,

    /// only return audit log entries with these types
    #[cfg_attr(feature = "serde", serde(default, rename = "type"))]
    pub ty: Vec<String>,

    // TODO: implement
    /// only return audit log entries with these statuses
    ///
    /// defaults to only `Success`
    #[cfg_attr(feature = "serde", serde(default))]
    pub status: Vec<AuditLogEntryStatus>,
}

/// the status of an audit log event
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AuditLogEntryStatus {
    /// the operation was successful
    Success,

    /// the operation was blocked at the user did not have permission
    Unauthorized,

    /// the operation failed to succeed
    Failed,
}

// TODO: use this instead of PaginationResponse<AuditLogEntry>
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AuditLogPaginationResponse {
    /// the audit log entries themselves
    pub audit_log_entries: Vec<AuditLogEntry>,

    /// threads referenced in the audit log entries
    pub threads: Vec<Channel>,

    /// users referenced in the audit log entries
    ///
    /// this includes actors (ie. users who did actions) and targets (ie. users who were affected by actions)
    pub users: Vec<User>,

    /// room members referenced in the audit log entries
    ///
    /// this includes actors (ie. room members who did actions) and targets (ie. room members who were affected by actions)
    pub room_members: Vec<RoomMember>,

    /// webhooks referenced in the audit log entries
    pub webhooks: Vec<Webhook>,

    // TODO: include calendar events, calendar overwrites, automod rules, integrations, etc...
    /// whether there are more audit log events that can be fetched
    pub has_more: bool,

    /// pagination cursor
    pub cursor: Option<String>,
}

impl AuditLogEntry {
    #[inline]
    pub fn strip_request_metadata(&mut self) {
        self.ip_addr = None;
        self.user_agent = None;
    }

    #[inline]
    pub fn strip_session(&mut self) {
        self.session_id = None;
    }

    /// get the duration of this request
    ///
    /// this is the time elapsed between `started_at` and `ended_at`
    pub fn duration(&self) -> Duration {
        (self.ended_at - self.started_at)
            .try_into()
            .unwrap_or_default()
    }
}

impl AuditLogEntryType {
    /// if this is a room event
    ///
    /// for example: RoomUpdate, Role events, Channel events, etc
    pub fn is_room(&self) -> bool {
        use AuditLogEntryType::*;
        matches!(
            self,
            RoomCreate { .. }
                | RoomUpdate { .. }
                | RoomDelete { .. }
                | RoomUndelete { .. }
                | RoomQuarantine { .. }
                | RoomUnquarantine { .. }
                | ChannelCreate { .. }
                | ChannelUpdate { .. }
                | ChannelReorder { .. }
                | ChannelReindex { .. }
                | MessageDelete { .. }
                | MessageVersionDelete { .. }
                | MessageDeleteBulk { .. }
                | MessageRemove { .. }
                | MessageRestore { .. }
                | RoleCreate { .. }
                | RoleUpdate { .. }
                | RoleDelete { .. }
                | RoleReorder { .. }
                | InviteCreate { .. }
                | InviteUpdate { .. }
                | InviteDelete { .. }
                | ReactionPurge { .. }
                | ReactionDeleteAll { .. }
                | ReactionDeleteKey { .. }
                | ReactionDeleteUser { .. }
                | EmojiCreate { .. }
                | EmojiUpdate { .. }
                | EmojiDelete { .. }
                | PermissionOverwriteSet { .. }
                | PermissionOverwriteDelete { .. }
                | MemberKick { .. }
                | MemberBan { .. }
                | MemberUnban { .. }
                | MemberPrune { .. }
                | MemberUpdate { .. }
                | MemberDisconnect { .. }
                | MemberDisconnectAll { .. }
                | MemberMove { .. }
                | RoleApply { .. }
                | RoleUnapply { .. }
                | BotAdd { .. }
                | ThreadMemberAdd { .. }
                | ThreadMemberRemove { .. }
                | MessagePin { .. }
                | MessageUnpin { .. }
                | MessagePinReorder { .. }
                | CalendarEventCreate { .. }
                | CalendarEventUpdate { .. }
                | CalendarEventDelete { .. }
                | CalendarOverwriteCreate { .. }
                | CalendarOverwriteUpdate { .. }
                | CalendarOverwriteDelete { .. }
                | CalendarRsvpDelete { .. }
                | WebhookCreate { .. }
                | WebhookUpdate { .. }
                | WebhookDelete { .. }
                | RatelimitUpdate { .. }
                | RatelimitDelete { .. }
                | RatelimitDeleteAll { .. }
                | AutomodRuleCreate { .. }
                | AutomodRuleUpdate { .. }
                | AutomodRuleDelete { .. }
        )
    }

    /// if this is a server event
    ///
    /// for example: Admin events, etc
    ///
    /// does not include room events for the server room
    pub fn is_server(&self) -> bool {
        use AuditLogEntryType::*;
        matches!(
            self,
            AdminWhisper { .. } | AdminBroadcast { .. } | ServerUpdate { .. }
        )
    }

    /// if this is a user event
    ///
    /// for example: UserUpdate, Session events, etc
    pub fn is_user(&self) -> bool {
        use AuditLogEntryType::*;
        matches!(
            self,
            UserUpdate { .. }
                | UserSuspend { .. }
                | UserUnsuspend { .. }
                | UserRegistered { .. }
                | UserDelete { .. }
                | UserUndelete { .. }
                | FriendRequest { .. }
                | FriendAccept { .. }
                | FriendDelete { .. }
                | BlockCreate { .. }
                | BlockDelete { .. }
                | IgnoreAdd { .. }
                | IgnoreRemove { .. }
                | SessionLogin { .. }
                | SessionUpdate { .. }
                | SessionDelete { .. }
                | SessionDeleteAll
                | AuthUpdate { .. }
                | AuthSudo { .. }
                | EmailCreate { .. }
                | EmailUpdate { .. }
                | EmailDelete { .. }
                | ConnectionCreate { .. }
                | ConnectionDelete { .. }
                | HarvestCreate { .. }
        )
    }

    /// if this is a application event
    ///
    /// for example: ApplicationUpdate, Emoji events, etc
    pub fn is_application(&self) -> bool {
        use AuditLogEntryType::*;
        matches!(
            self,
            ApplicationCreate { .. } | ApplicationUpdate { .. } | ApplicationDelete { .. }
        )
    }
}
