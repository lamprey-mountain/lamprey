#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::error::SyncError;

use crate::v1::types::{
    application::{Application, Connection},
    automod::{AutomodRule, AutomodRuleExecution},
    document::{DocumentBranch, DocumentStateVector, DocumentTag, DocumentUpdate},
    presence::Presence,
    util::Time,
    voice::Call,
    webhook::Webhook,
    ApplicationId, AuditLogEntry, AutomodRuleId, CalendarEventId, ConnectionId, DocumentBranchId,
    DocumentTagId, InviteTargetId, InviteWithMetadata, Relationship, RoomBan, ThreadMember,
    WebhookId,
};

use crate::v2::types::message::Message as MessageV2;

use super::{
    calendar::{CalendarEvent, CalendarEventParticipant, CalendarOverwrite},
    emoji::EmojiCustom,
    harvest::Harvest,
    notifications::{Notification, NotificationFlush, NotificationMarkRead},
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    reaction::ReactionKey,
    role::RoleReorderItem,
    tag::Tag,
    voice::{SignallingMessage, VoiceState},
    Channel, ChannelId, EmojiId, InviteCode, MessageId, MessageVerId, Role, RoleId, Room, RoomId,
    RoomMember, Session, SessionId, SessionToken, TagId, User, UserId,
};

// TODO: encode binary data as base64 for json, binary for msgpack

// TODO: include nonce/seq for MessageClient too, so theres some way to associate an error response to a request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MessageClient {
    /// initial message
    Hello {
        token: SessionToken,

        presence: Option<Presence>,

        #[cfg_attr(feature = "serde", serde(flatten))]
        resume: Option<SyncResume>,
    },

    /// set presence
    Presence { presence: Presence },

    /// heartbeat
    Pong,

    /// send arbitrary data to a voice server
    // NOTE: should i split this into multiple messages? i'll probably keep it how it is currently tbh
    // TODO: handle multiple connections/servers (or find out how to split one connection amongst multiple hosts?)
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
    },

    /// subscribe to a range of room or thread members. you can subscribe to one list at a time.
    MemberListSubscribe {
        // TODO: rename thread_id -> channel_id
        // EXACTLY one of room_id or thread_id must be provided
        room_id: Option<RoomId>,
        thread_id: Option<ChannelId>,

        /// the ranges to subscribe to
        ranges: Vec<(u64, u64)>,
    },

    DocumentSubscribe {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        state_vector: Option<DocumentStateVector>,
        // TODO: subscribing to multiple documents at once
        // channel_ids: Vec<ChannelId>,
    },

    /// edit a document
    ///
    /// must be subscribed via DocumentSubscribe
    DocumentEdit {
        /// the document thats being edited
        channel_id: ChannelId,

        branch_id: DocumentBranchId,

        /// the encoded update to this document
        update: DocumentUpdate,
    },

    /// update your document presence
    ///
    /// must be subscribed via DocumentSubscribe
    DocumentPresence {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        cursor_head: String,
        cursor_tail: Option<String>,
    },

    // TODO: centralize into one single Subscribe message
    #[cfg(any())]
    /// subscribe to some resources
    Subscribe(SyncSubscribe),
}

/// update what the client is subscribed to
///
/// leaving a field as None will skip updating. set it to an empty vec to clear.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SyncSubscribe {
    /// the member lists to subscribe to
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub member_lists: Option<Vec<SyncSubscribeMemberList>>,

    /// the documents to subscribe to
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub documents: Option<Vec<SyncSubscribeDocument>>,
    // TODO: subscribing to user profiles
    // TODO: subscribing to invites
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscribeMemberList {
    pub room_id: Option<RoomId>,

    // renamed from thread_id
    pub channel_id: Option<ChannelId>,

    /// the ranges to subscribe to
    pub ranges: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscribeDocument {
    pub channel_id: ChannelId,
    pub branch_id: DocumentBranchId,
    pub state_vector: Option<DocumentStateVector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncResume {
    pub conn: ConnectionId,
    pub seq: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageEnvelope {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload: MessagePayload,
    // should i move seq here?
}

// NOTE: consider making Ready and ReadySupplemental part of Sync
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "op"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessagePayload {
    /// heartbeat
    Ping,

    /// data to keep local copy of state in sync with server
    Sync {
        /// the data for this sync event
        data: Box<MessageSync>,

        /// the sequence number of this event, for resuming
        seq: u64,

        /// the nonce, if this is in response to a request with the `Idempotency-Key` header set
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        nonce: Option<String>,
    },

    /// some kind of error
    Error {
        error: String,
        code: Option<SyncError>,
    },

    /// successfully connected
    Ready {
        /// current user, null if session is unauthed
        user: Option<Box<User>>,

        /// the application that's being used
        ///
        /// - if this is a bot, this is the bot application
        /// - if this is an oauth app, this is the oauth application
        application: Option<Box<Application>>,

        /// current session
        session: Session,

        /// connection id
        conn: ConnectionId,

        /// sequence id for reconnecting
        seq: u64,
    },

    /// send all missed messages, now tailing live event stream
    Resumed,

    /// client needs to disconnect and reconnect
    Reconnect {
        /// whether the client can resume
        can_resume: bool,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MessageSync {
    // TODO: move Ready here
    // /// successfully connected
    // Ready {
    //     /// current user, null if session is unauthed
    //     user: Box<Option<User>>,

    //     // /// the application this bot user belongs, if the user is a bot
    //     // application: Box<Option<Application>>,
    //     /// current session
    //     session: Session,

    //     /// connection id
    //     conn: String,
    // },
    /// extra data for the client to function, sent after Ready
    // NOTE: should this be included for bots?
    Ambient {
        /// the user that this Ambient message is for
        user_id: UserId,

        /// all rooms the user can see
        rooms: Vec<Room>,

        /// all roles in all rooms the user can see
        roles: Vec<Role>,

        /// all non-thread channels the user can see
        channels: Vec<Channel>,

        /// all active (ie. not archived) threads the user can see
        threads: Vec<Channel>,

        /// the user's room member object for each room the user is in
        room_members: Vec<RoomMember>,

        /// user's global preferences
        config: PreferencesGlobal,
        // NOTE: maybe i should include even more data
        // - friends/relationships (including friend requests)
        // - dms
        // - emoji
    },

    RoomCreate {
        room: Room,
    },

    // RoomCreate2 {
    //     room: Room,
    //     roles: Vec<Role>,
    //     channels: Vec<Channel>,
    //     threads: Vec<Channel>,
    //     room_member: Option<RoomMember>,
    // },
    RoomUpdate {
        room: Room,
    },

    RoomDelete {
        room_id: RoomId,
    },

    ChannelCreate {
        channel: Box<Channel>,
    },

    ChannelUpdate {
        channel: Box<Channel>,
    },

    ChannelTyping {
        channel_id: ChannelId,
        user_id: UserId,
        until: Time,
    },

    /// read receipt update
    ChannelAck {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    // ThreadCreate {
    //     thread: Box<Channel>,
    // },

    // ThreadUpdate {
    //     thread: Box<Channel>,
    // },

    // ThreadDelete {
    //     thread_id: ChannelId,
    // },
    MessageCreate {
        // i know, it's cursed to return v2 messages in a v1 api. but this is still in pre alpha so i don't really care.
        message: MessageV2,
        // /// the room member of the author, if this was sent in a room
        // room_member: Option<RoomMember>,

        // /// the thread member of the author, if this was sent in a thread
        // thread_member: Option<ThreadMember>,

        // /// the user who sent this message
        // user: User,
    },

    MessageUpdate {
        message: MessageV2,
        // /// the room member of the author, if this was sent in a room
        // room_member: Option<RoomMember>,

        // /// the thread member of the author, if this was sent in a thread
        // thread_member: Option<ThreadMember>,

        // /// the user who sent this message
        // user: User,
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

    /// delete multiple messages at once
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

        // TODO: remove `message_ids`
        message_ids: Vec<MessageId>,
        // TODO: add `messages`
        // messages: Vec<Message>,
    },

    HarvestUpdate {
        harvest: Harvest,
    },

    RoomMemberCreate {
        member: RoomMember,
        user: User,
    },

    RoomMemberUpdate {
        member: RoomMember,
        user: User,
    },

    RoomMemberDelete {
        room_id: RoomId,
        user_id: UserId,
    },

    ThreadMemberUpsert {
        room_id: Option<RoomId>,
        thread_id: ChannelId,

        /// members that were added to the thread
        added: Vec<ThreadMember>,

        /// members that were removed from the thread
        removed: Vec<UserId>,
    },

    RoleCreate {
        role: Role,
    },

    RoleUpdate {
        role: Role,
    },

    RoleDelete {
        room_id: RoomId,
        role_id: RoleId,
    },

    RoleReorder {
        room_id: RoomId,
        roles: Vec<RoleReorderItem>,
    },

    InviteCreate {
        invite: Box<InviteWithMetadata>,
    },

    InviteUpdate {
        invite: Box<InviteWithMetadata>,
    },

    InviteDelete {
        code: InviteCode,
        target: InviteTargetId,
        // TODO: should i remove this?
        creator_id: UserId,
    },

    ReactionCreate {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove one specific emoji on a message
    ReactionDelete {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions for a reaction key on a message
    ReactionDeleteKey {
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions on a message
    ReactionDeleteAll {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    EmojiCreate {
        emoji: EmojiCustom,
    },

    EmojiUpdate {
        emoji: EmojiCustom,
    },

    EmojiDelete {
        emoji_id: EmojiId,
        room_id: RoomId,
    },

    TagCreate {
        tag: Tag,
    },

    TagUpdate {
        tag: Tag,
    },

    TagDelete {
        channel_id: ChannelId,
        tag_id: TagId,
    },

    /// receive a signalling message from a voice server
    VoiceDispatch {
        /// who to send this dispatch to
        user_id: UserId,
        payload: SignallingMessage,
    },

    VoiceState {
        user_id: UserId,
        state: Option<VoiceState>,

        // HACK: make it possible to use this for auth checks
        #[cfg_attr(feature = "serde", serde(skip))]
        old_state: Option<VoiceState>,
    },

    CallCreate {
        call: Call,
    },

    CallUpdate {
        call: Call,
    },

    CallDelete {
        // TODO: add room_id: Option<RoomId>,
        channel_id: ChannelId,
    },

    UserCreate {
        user: User,
    },

    UserUpdate {
        user: User,
    },

    PresenceUpdate {
        user_id: UserId,
        presence: Presence,
    },

    PreferencesGlobal {
        user_id: UserId,
        config: PreferencesGlobal,
    },

    PreferencesRoom {
        user_id: UserId,
        room_id: RoomId,
        config: PreferencesRoom,
    },

    PreferencesChannel {
        user_id: UserId,
        channel_id: ChannelId,
        config: PreferencesChannel,
    },

    PreferencesUser {
        user_id: UserId,
        target_user_id: UserId,
        config: PreferencesUser,
    },

    UserDelete {
        id: UserId,
    },

    SessionCreate {
        session: Session,
    },

    SessionUpdate {
        session: Session,
    },

    SessionDelete {
        id: SessionId,
        user_id: Option<UserId>,
    },

    SessionDeleteAll {
        user_id: UserId,
    },

    RelationshipUpsert {
        user_id: UserId,
        target_user_id: UserId,
        relationship: Relationship,
    },

    RelationshipDelete {
        user_id: UserId,
        target_user_id: UserId,
    },

    ConnectionCreate {
        user_id: UserId,
        connection: Connection,
    },

    ConnectionDelete {
        user_id: UserId,
        app_id: ApplicationId,
    },

    AuditLogEntryCreate {
        entry: AuditLogEntry,
    },

    BanCreate {
        room_id: RoomId,
        ban: RoomBan,
    },

    BanDelete {
        room_id: RoomId,
        user_id: UserId,
    },

    MemberListSync {
        /// which user this list sync is for
        user_id: UserId,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
        ops: Vec<MemberListOp>,
        groups: Vec<MemberListGroup>,
    },

    InboxNotificationCreate {
        user_id: UserId,
        notification: Notification,
    },

    InboxMarkRead {
        user_id: UserId,
        #[cfg_attr(feature = "serde", serde(flatten))]
        params: NotificationMarkRead,
    },

    InboxMarkUnread {
        user_id: UserId,
        #[cfg_attr(feature = "serde", serde(flatten))]
        params: NotificationMarkRead,
    },

    InboxFlush {
        user_id: UserId,
        #[cfg_attr(feature = "serde", serde(flatten))]
        params: NotificationFlush,
    },

    CalendarEventCreate {
        event: CalendarEvent,
    },

    CalendarEventUpdate {
        event: CalendarEvent,
    },

    CalendarEventDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
    },

    CalendarOverwriteCreate {
        channel_id: ChannelId,
        overwrite: CalendarOverwrite,
    },

    CalendarOverwriteUpdate {
        channel_id: ChannelId,
        overwrite: CalendarOverwrite,
    },

    CalendarOverwriteDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        seq: u64,
    },

    CalendarRsvpCreate {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        participant: CalendarEventParticipant,
    },

    CalendarRsvpDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        user_id: UserId,
    },

    CalendarOverwriteRsvpCreate {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        seq: u64,
        participant: CalendarEventParticipant,
    },

    CalendarOverwriteRsvpDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
    },

    WebhookCreate {
        webhook: Webhook,
    },

    WebhookUpdate {
        webhook: Webhook,
    },

    WebhookDelete {
        webhook_id: WebhookId,
        room_id: Option<RoomId>,
        channel_id: ChannelId,
    },

    // TODO: split out AutomodManage with RoomManage?
    /// an auto moderation rule was created. only sent to users with RoomManage.
    AutomodRuleCreate {
        rule: AutomodRule,
    },

    /// an auto moderation rule was updated. only sent to users with RoomManage.
    AutomodRuleUpdate {
        rule: AutomodRule,
    },

    /// an auto moderation rule was deleted. only sent to users with RoomManage.
    AutomodRuleDelete {
        rule_id: AutomodRuleId,
        room_id: RoomId,
    },

    /// an auto moderation rule was executed. only sent to users with RoomManage.
    AutomodRuleExecute {
        execution: AutomodRuleExecution,
    },

    RatelimitUpdate {
        channel_id: ChannelId,
        user_id: UserId,
        slowmode_thread_expire_at: Option<Time>,
        slowmode_message_expire_at: Option<Time>,
    },

    /// an edit to a document
    ///
    /// only returned if subscribed
    DocumentEdit {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,

        /// the encoded update to this document
        update: DocumentUpdate,
    },

    /// user presence in a document
    ///
    /// only returned if subscribed
    DocumentPresence {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        user_id: UserId,
        cursor_head: String,
        cursor_tail: Option<String>,
    },

    /// confirmation that the client is now subscribed to a document.
    ///
    /// sent after the initial `DocumentEdit` containing the current document
    /// state has been sent. clients should wait for this event before sending
    /// `DocumentPresence` or `DocumentEdit` messages to avoid "not subscribed" errors.
    DocumentSubscribed {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        /// the connection ID this subscription confirmation is sent to
        connection_id: ConnectionId,
    },

    DocumentTagCreate {
        channel_id: ChannelId,
        tag: DocumentTag,
    },

    DocumentTagUpdate {
        channel_id: ChannelId,
        tag: DocumentTag,
    },

    DocumentTagDelete {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        tag_id: DocumentTagId,
    },

    DocumentBranchCreate {
        branch: DocumentBranch,
    },

    DocumentBranchUpdate {
        branch: DocumentBranch,
    },

    // NOTE: currently unused, as branches are marked as closed/merged rather than deleted
    // how do i want to handle branch deletions? i want to clean up old editing contexts. maybe once closed/merged, make branches readonly and delete the associated editing context
    DocumentBranchDelete {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
    },

    /// A piece of media has processed and is now in the `Uploaded` state.
    MediaProcessed {
        media: crate::v2::types::media::Media,
        session_id: SessionId,
    },

    MediaUpdate {
        media: crate::v2::types::media::Media,
    },
}

// TODO: skip sending room_members/thread_members/users if the client already has them
// TODO: move member list stuff to a submodule
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MemberListOp {
    /// replace a range of members
    Sync {
        /// the start of the range
        position: u64,

        /// the users in this range
        items: Vec<UserId>,

        /// only returned if channel is in a room and not already cached by client
        room_members: Option<Vec<RoomMember>>,

        /// only returned if listing members in a thread and not already cached by client
        thread_members: Option<Vec<ThreadMember>>,

        /// users in this range that are not already cached by client
        users: Option<Vec<User>>,
    },

    /// insert a member
    Insert {
        position: u64,
        user_id: UserId,
        room_member: Option<RoomMember>,
        thread_member: Option<ThreadMember>,
        user: Option<Box<User>>,
    },

    /// delete a range of one or more members
    Delete {
        position: u64,
        // usually will be 1
        count: u64,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MemberListGroup {
    pub id: MemberListGroupId,
    pub count: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberListGroupId {
    /// online members without a hoisted role
    Online,

    /// offline members, including those with a role
    Offline,

    /// hoisted roles
    #[cfg_attr(feature = "serde", serde(untagged))]
    Role(RoleId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct SyncParams {
    pub version: SyncVersion,
    pub compression: Option<SyncCompression>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub format: SyncFormat,
}

// i thought that putting the api version in the path would be better, but
// apparently websockets are hard to load balance. being able to use arbitrary
// urls/paths in the future could be helpful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[repr(u8)]
pub enum SyncVersion {
    V1 = 1,
}

#[cfg(feature = "serde")]
impl Serialize for SyncVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SyncVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            1 => Ok(SyncVersion::V1),
            n => Err(serde::de::Error::unknown_variant(&n.to_string(), &["1"])),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncFormat {
    #[default]
    Json,
    Msgpack,
}

/// how data should be compressed
///
/// the client may send non-compressed json, but not non-compressed
/// msgpack payloads (as theres no way to differentiate between compressed and
/// non-compressed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncCompression {
    /// Deflate compression
    Deflate,
}

impl MessageSync {
    /// the id of the room this message was emitted in, if it is known
    pub fn room_id(&self) -> Option<RoomId> {
        match self {
            Self::Ambient { .. } => None,
            Self::RoomCreate { room } => Some(room.id),
            Self::RoomUpdate { room } => Some(room.id),
            Self::RoomDelete { room_id } => Some(*room_id),
            Self::ChannelCreate { channel } => channel.room_id,
            Self::ChannelUpdate { channel } => channel.room_id,
            Self::ChannelTyping { .. } => None,
            Self::ChannelAck { .. } => None,
            Self::MessageCreate { message } => message.room_id,
            Self::MessageUpdate { message } => message.room_id,
            Self::MessageDelete { .. } => None,
            Self::MessageVersionDelete { .. } => None,
            Self::MessageDeleteBulk { .. } => None,
            Self::MessageRemove { .. } => None,
            Self::MessageRestore { .. } => None,
            Self::HarvestUpdate { .. } => None,
            Self::RoomMemberCreate { member, .. } => Some(member.room_id),
            Self::RoomMemberUpdate { member, .. } => Some(member.room_id),
            Self::RoomMemberDelete { room_id, .. } => Some(*room_id),
            Self::ThreadMemberUpsert { room_id, .. } => *room_id,
            Self::RoleCreate { role } => Some(role.room_id),
            Self::RoleUpdate { role } => Some(role.room_id),
            Self::RoleDelete { room_id, .. } => Some(*room_id),
            Self::RoleReorder { room_id, .. } => Some(*room_id),
            Self::InviteCreate { invite } => match &invite.invite.target {
                super::invite::InviteTarget::Room { room, .. } => Some(room.id),
                _ => None,
            },
            Self::InviteUpdate { invite } => match &invite.invite.target {
                super::invite::InviteTarget::Room { room, .. } => Some(room.id),
                _ => None,
            },
            Self::InviteDelete { target, .. } => match target {
                InviteTargetId::Room { room_id, .. } => Some(*room_id),
                _ => None,
            },
            Self::ReactionCreate { .. } => None,
            Self::ReactionDelete { .. } => None,
            Self::ReactionDeleteKey { .. } => None,
            Self::ReactionDeleteAll { .. } => None,
            Self::EmojiCreate { emoji } => match &emoji.owner {
                Some(super::emoji::EmojiOwner::Room { room_id }) => Some(*room_id),
                _ => None,
            },
            Self::EmojiUpdate { emoji } => match &emoji.owner {
                Some(super::emoji::EmojiOwner::Room { room_id }) => Some(*room_id),
                _ => None,
            },
            Self::EmojiDelete { room_id, .. } => Some(*room_id),
            Self::TagCreate { .. } => None,
            Self::TagUpdate { .. } => None,
            Self::TagDelete { .. } => None,
            Self::VoiceDispatch { .. } => None,
            Self::VoiceState { .. } => {
                // we don't know the room_id easily here without more context, but usually voice is in a room
                None
            }
            Self::CallCreate { .. } => None,
            Self::CallUpdate { .. } => None,
            Self::CallDelete { .. } => None,
            Self::UserCreate { .. } => None,
            Self::UserUpdate { .. } => None,
            Self::PresenceUpdate { .. } => None,
            Self::PreferencesGlobal { .. } => None,
            Self::PreferencesRoom { room_id, .. } => Some(*room_id),
            Self::PreferencesChannel { .. } => None,
            Self::PreferencesUser { .. } => None,
            Self::UserDelete { .. } => None,
            Self::SessionCreate { .. } => None,
            Self::SessionUpdate { .. } => None,
            Self::SessionDelete { .. } => None,
            Self::SessionDeleteAll { .. } => None,
            Self::RelationshipUpsert { .. } => None,
            Self::RelationshipDelete { .. } => None,
            Self::ConnectionCreate { .. } => None,
            Self::ConnectionDelete { .. } => None,
            Self::AuditLogEntryCreate { entry } => Some(entry.room_id),
            Self::BanCreate { room_id, .. } => Some(*room_id),
            Self::BanDelete { room_id, .. } => Some(*room_id),
            Self::MemberListSync { room_id, .. } => *room_id,
            Self::InboxNotificationCreate { .. } => None,
            Self::InboxMarkRead { .. } => None,
            Self::InboxMarkUnread { .. } => None,
            Self::InboxFlush { .. } => None,
            Self::CalendarEventCreate { .. } => None,
            Self::CalendarEventUpdate { .. } => None,
            Self::CalendarEventDelete { .. } => None,
            Self::CalendarOverwriteCreate { .. } => None,
            Self::CalendarOverwriteUpdate { .. } => None,
            Self::CalendarOverwriteDelete { .. } => None,
            Self::CalendarRsvpCreate { .. } => None,
            Self::CalendarRsvpDelete { .. } => None,
            Self::CalendarOverwriteRsvpCreate { .. } => None,
            Self::CalendarOverwriteRsvpDelete { .. } => None,
            Self::WebhookCreate { webhook } => webhook.room_id,
            Self::WebhookUpdate { webhook } => webhook.room_id,
            Self::WebhookDelete { room_id, .. } => *room_id,
            Self::AutomodRuleCreate { rule } => Some(rule.room_id),
            Self::AutomodRuleUpdate { rule } => Some(rule.room_id),
            Self::AutomodRuleDelete { room_id, .. } => Some(*room_id),
            Self::AutomodRuleExecute { execution } => Some(execution.rule.room_id),
            Self::RatelimitUpdate { .. } => None,
            Self::DocumentEdit { .. } => None,
            Self::DocumentPresence { .. } => None,
            Self::DocumentSubscribed { .. } => None,
            Self::DocumentTagCreate { .. } => None,
            Self::DocumentTagUpdate { .. } => None,
            Self::DocumentTagDelete { .. } => None,
            Self::DocumentBranchCreate { .. } => None,
            Self::DocumentBranchUpdate { .. } => None,
            Self::DocumentBranchDelete { .. } => None,
            Self::MediaProcessed { .. } => None,
            Self::MediaUpdate { .. } => None,
        }
    }

    /// the id of the channel this message was emitted in, if it is known
    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            Self::Ambient { .. } => None,
            Self::RoomCreate { .. } => None,
            Self::RoomUpdate { .. } => None,
            Self::RoomDelete { .. } => None,
            Self::ChannelCreate { channel } => Some(channel.id),
            Self::ChannelUpdate { channel } => Some(channel.id),
            Self::ChannelTyping { channel_id, .. } => Some(*channel_id),
            Self::ChannelAck { channel_id, .. } => Some(*channel_id),
            Self::MessageCreate { message } => Some(message.channel_id),
            Self::MessageUpdate { message } => Some(message.channel_id),
            Self::MessageDelete { channel_id, .. } => Some(*channel_id),
            Self::MessageVersionDelete { channel_id, .. } => Some(*channel_id),
            Self::MessageDeleteBulk { channel_id, .. } => Some(*channel_id),
            Self::MessageRemove { channel_id, .. } => Some(*channel_id),
            Self::MessageRestore { channel_id, .. } => Some(*channel_id),
            Self::HarvestUpdate { .. } => None,
            Self::RoomMemberCreate { .. } => None,
            Self::RoomMemberUpdate { .. } => None,
            Self::RoomMemberDelete { .. } => None,
            Self::ThreadMemberUpsert { thread_id, .. } => Some(*thread_id),
            Self::RoleCreate { .. } => None,
            Self::RoleUpdate { .. } => None,
            Self::RoleDelete { .. } => None,
            Self::RoleReorder { .. } => None,
            Self::InviteCreate { invite } => match &invite.invite.target {
                super::invite::InviteTarget::Room { channel, .. } => channel.as_ref().map(|c| c.id),
                super::invite::InviteTarget::Gdm { channel } => Some(channel.id),
                _ => None,
            },
            Self::InviteUpdate { invite } => match &invite.invite.target {
                super::invite::InviteTarget::Room { channel, .. } => channel.as_ref().map(|c| c.id),
                super::invite::InviteTarget::Gdm { channel } => Some(channel.id),
                _ => None,
            },
            Self::InviteDelete { target, .. } => match target {
                InviteTargetId::Room { channel_id, .. } => *channel_id,
                InviteTargetId::Gdm { channel_id } => Some(*channel_id),
                _ => None,
            },
            Self::ReactionCreate { channel_id, .. } => Some(*channel_id),
            Self::ReactionDelete { channel_id, .. } => Some(*channel_id),
            Self::ReactionDeleteKey { channel_id, .. } => Some(*channel_id),
            Self::ReactionDeleteAll { channel_id, .. } => Some(*channel_id),
            Self::EmojiCreate { .. } => None,
            Self::EmojiUpdate { .. } => None,
            Self::EmojiDelete { .. } => None,
            Self::TagCreate { tag } => Some(tag.channel_id),
            Self::TagUpdate { tag } => Some(tag.channel_id),
            Self::TagDelete { channel_id, .. } => Some(*channel_id),
            Self::VoiceDispatch { .. } => None,
            Self::VoiceState { state, .. } => state.as_ref().map(|s| s.channel_id),
            Self::CallCreate { call } => Some(call.channel_id),
            Self::CallUpdate { call } => Some(call.channel_id),
            Self::CallDelete { channel_id } => Some(*channel_id),
            Self::UserCreate { .. } => None,
            Self::UserUpdate { .. } => None,
            Self::PresenceUpdate { .. } => None,
            Self::PreferencesGlobal { .. } => None,
            Self::PreferencesRoom { .. } => None,
            Self::PreferencesChannel { channel_id, .. } => Some(*channel_id),
            Self::PreferencesUser { .. } => None,
            Self::UserDelete { .. } => None,
            Self::SessionCreate { .. } => None,
            Self::SessionUpdate { .. } => None,
            Self::SessionDelete { .. } => None,
            Self::SessionDeleteAll { .. } => None,
            Self::RelationshipUpsert { .. } => None,
            Self::RelationshipDelete { .. } => None,
            Self::ConnectionCreate { .. } => None,
            Self::ConnectionDelete { .. } => None,
            Self::AuditLogEntryCreate { .. } => None,
            Self::BanCreate { .. } => None,
            Self::BanDelete { .. } => None,
            Self::MemberListSync { channel_id, .. } => *channel_id,
            Self::InboxNotificationCreate { .. } => None,
            Self::InboxMarkRead { .. } => None,
            Self::InboxMarkUnread { .. } => None,
            Self::InboxFlush { .. } => None,
            Self::CalendarEventCreate { event } => Some(event.channel_id),
            Self::CalendarEventUpdate { event } => Some(event.channel_id),
            Self::CalendarEventDelete { channel_id, .. } => Some(*channel_id),
            Self::CalendarOverwriteCreate { channel_id, .. } => Some(*channel_id),
            Self::CalendarOverwriteUpdate { channel_id, .. } => Some(*channel_id),
            Self::CalendarOverwriteDelete { channel_id, .. } => Some(*channel_id),
            Self::CalendarRsvpCreate { channel_id, .. } => Some(*channel_id),
            Self::CalendarRsvpDelete { channel_id, .. } => Some(*channel_id),
            Self::CalendarOverwriteRsvpCreate { channel_id, .. } => Some(*channel_id),
            Self::CalendarOverwriteRsvpDelete { channel_id, .. } => Some(*channel_id),
            Self::WebhookCreate { webhook } => Some(webhook.channel_id),
            Self::WebhookUpdate { webhook } => Some(webhook.channel_id),
            Self::WebhookDelete { channel_id, .. } => Some(*channel_id),
            Self::AutomodRuleCreate { .. } => None,
            Self::AutomodRuleUpdate { .. } => None,
            Self::AutomodRuleDelete { .. } => None,
            Self::AutomodRuleExecute { .. } => None,
            Self::RatelimitUpdate { channel_id, .. } => Some(*channel_id),
            Self::DocumentEdit { channel_id, .. } => Some(*channel_id),
            Self::DocumentPresence { channel_id, .. } => Some(*channel_id),
            Self::DocumentSubscribed { channel_id, .. } => Some(*channel_id),
            Self::DocumentTagCreate { channel_id, .. } => Some(*channel_id),
            Self::DocumentTagUpdate { channel_id, .. } => Some(*channel_id),
            Self::DocumentTagDelete { channel_id, .. } => Some(*channel_id),
            Self::DocumentBranchCreate { branch } => Some(branch.document_id),
            Self::DocumentBranchUpdate { branch } => Some(branch.document_id),
            Self::DocumentBranchDelete { channel_id, .. } => Some(*channel_id),
            Self::MediaProcessed { .. } => None,
            Self::MediaUpdate { .. } => None,
        }
    }
}
