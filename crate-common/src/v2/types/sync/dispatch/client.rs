use super::invite::{InviteCreate, InviteDelete, InviteDeleteBulk, InviteUpdate};
use crate::v1::types::{
    application::Connection,
    harvest::Harvest,
    mirror::seq::ClientSeq,
    misc::Time,
    notifications::{Notification, NotificationFlush, NotificationMarkRead},
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    session::Session,
    user::Relationship,
    voice::VoiceState,
    ApplicationId, ChannelId, MessageId, RoomId, SessionId, UserId,
};
use lamprey_macros::record;

/// something happened to the client user
#[record]
#[serde(tag = "type")]
pub enum DispatchClient {
    HarvestUpdate(HarvestUpdate),
    PreferencesGlobalUpdate(PreferencesGlobalUpdate),
    PreferencesRoomUpdate(PreferencesRoomUpdate),
    PreferencesChannelUpdate(PreferencesChannelUpdate),
    PreferencesUserUpdate(PreferencesUserUpdate),
    ReadStateUpdate(ReadStateUpdate),
    PassiveRoomUpdate(PassiveRoomUpdate),
    SessionCreate(SessionCreate),
    SessionUpdate(SessionUpdate),
    SessionDelete(SessionDelete),
    SessionDeleteAll(SessionDeleteAll),
    RelationshipUpsert(RelationshipUpsert),
    RelationshipDelete(RelationshipDelete),
    ConnectionCreate(ConnectionCreate),
    ConnectionDelete(ConnectionDelete),
    InboxNotificationCreate(InboxNotificationCreate),
    InboxMarkRead(InboxMarkRead),
    InboxMarkUnread(InboxMarkUnread),
    InboxFlush(InboxFlush),
    RatelimitUpdate(RatelimitUpdate),
    InviteCreate(ClientInviteCreate),
    InviteUpdate(ClientInviteUpdate),
    InviteDelete(ClientInviteDelete),
    InviteDeleteBulk(ClientInviteDeleteBulk),
}

#[record]
pub struct HarvestUpdate {
    pub harvest: Box<Harvest>,
    pub seq: ClientSeq,
}

#[record]
pub struct PreferencesGlobalUpdate {
    pub config: Box<PreferencesGlobal>,
    pub seq: ClientSeq,
}

/// a user's read states were updated
#[record]
pub struct ReadStateUpdate {
    pub read_states: Vec<ReadState>,
    pub seq: ClientSeq,
}

/// a user's read state for a channel
#[record]
#[derive(Default)]
pub struct ReadState {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_read_id: Option<MessageId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_read_at: Option<Time>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention_count: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pins_read_at: Option<Time>,
}

impl DispatchClient {
    /// the client sync sequence number of this event
    pub fn seq(&self) -> ClientSeq {
        match self {
            DispatchClient::HarvestUpdate(a) => a.seq,
            DispatchClient::PreferencesGlobalUpdate(a) => a.seq,
            DispatchClient::PreferencesRoomUpdate(a) => a.seq,
            DispatchClient::PreferencesChannelUpdate(a) => a.seq,
            DispatchClient::PreferencesUserUpdate(a) => a.seq,
            DispatchClient::ReadStateUpdate(a) => a.seq,
            DispatchClient::PassiveRoomUpdate(a) => a.seq,
            DispatchClient::SessionCreate(a) => a.seq,
            DispatchClient::SessionUpdate(a) => a.seq,
            DispatchClient::SessionDelete(a) => a.seq,
            DispatchClient::SessionDeleteAll(a) => a.seq,
            DispatchClient::RelationshipUpsert(a) => a.seq,
            DispatchClient::RelationshipDelete(a) => a.seq,
            DispatchClient::ConnectionCreate(a) => a.seq,
            DispatchClient::ConnectionDelete(a) => a.seq,
            DispatchClient::InboxNotificationCreate(a) => a.seq,
            DispatchClient::InboxMarkRead(a) => a.seq,
            DispatchClient::InboxMarkUnread(a) => a.seq,
            DispatchClient::InboxFlush(a) => a.seq,
            DispatchClient::RatelimitUpdate(a) => a.seq,
            DispatchClient::InviteCreate(a) => a.seq,
            DispatchClient::InviteUpdate(a) => a.seq,
            DispatchClient::InviteDelete(a) => a.seq,
            DispatchClient::InviteDeleteBulk(a) => a.seq,
        }
    }
}

/// a user's preferences in a room were updated
#[record]
pub struct PreferencesRoomUpdate {
    pub room_id: RoomId,
    pub config: Box<PreferencesRoom>,
    pub seq: ClientSeq,
}

/// a user's preferences in a channel were updated
#[record]
pub struct PreferencesChannelUpdate {
    pub channel_id: ChannelId,
    pub config: Box<PreferencesChannel>,
    pub seq: ClientSeq,
}

/// a user's preferences for another user were updated
#[record]
pub struct PreferencesUserUpdate {
    pub target_user_id: UserId,
    pub config: Box<PreferencesUser>,
    pub seq: ClientSeq,
}

/// a user's passive room state was updated
#[record]
pub struct PassiveRoomUpdate {
    pub room_id: RoomId,
    pub read_states: Vec<ReadState>,
    pub voice_states: Vec<VoiceState>,
    pub seq: ClientSeq,
}

/// a session was created
#[record]
pub struct SessionCreate {
    pub session: Box<Session>,
    pub seq: ClientSeq,
}

/// a session was updated
#[record]
pub struct SessionUpdate {
    pub session: Box<Session>,
    pub seq: ClientSeq,
}

/// a session was deleted
#[record]
pub struct SessionDelete {
    pub session_id: SessionId,
    pub user_id: Option<UserId>,
    pub seq: ClientSeq,
}

/// all sessions for a user were deleted
#[record]
pub struct SessionDeleteAll {
    pub user_id: UserId,
    pub seq: ClientSeq,
}

/// a relationship was created or updated
#[record]
pub struct RelationshipUpsert {
    pub target_user_id: UserId,
    pub relationship: Box<Relationship>,
    pub seq: ClientSeq,
}

/// a relationship was deleted
#[record]
pub struct RelationshipDelete {
    pub target_user_id: UserId,
    pub seq: ClientSeq,
}

/// an application connection was created
#[record]
pub struct ConnectionCreate {
    pub connection: Box<Connection>,
    pub seq: ClientSeq,
}

/// an application connection was deleted
#[record]
pub struct ConnectionDelete {
    pub app_id: ApplicationId,
    pub seq: ClientSeq,
}

/// a notification was added to the inbox
#[record]
pub struct InboxNotificationCreate {
    pub notification: Box<Notification>,
    pub seq: ClientSeq,
}

/// notifications were marked as read
#[record]
pub struct InboxMarkRead {
    #[serde(flatten)]
    pub params: NotificationMarkRead,
    pub seq: ClientSeq,
}

/// notifications were marked as unread
#[record]
pub struct InboxMarkUnread {
    #[serde(flatten)]
    pub params: NotificationMarkRead,
    pub seq: ClientSeq,
}

/// notifications were flushed
#[record]
pub struct InboxFlush {
    #[serde(flatten)]
    pub params: NotificationFlush,
    pub seq: ClientSeq,
}

/// a user's ratelimit state in a channel was updated
#[record]
pub struct RatelimitUpdate {
    pub channel_id: ChannelId,
    pub slowmode_thread_expire_at: Option<Time>,
    pub slowmode_message_expire_at: Option<Time>,
    pub seq: ClientSeq,
}

/// a user or server invite was created
#[record]
pub struct ClientInviteCreate {
    #[serde(flatten)]
    pub invite: InviteCreate,
    pub seq: ClientSeq,
}

/// a user or server invite was updated
#[record]
pub struct ClientInviteUpdate {
    #[serde(flatten)]
    pub invite: InviteUpdate,
    pub seq: ClientSeq,
}

/// a user or server invite was deleted
#[record]
pub struct ClientInviteDelete {
    #[serde(flatten)]
    pub invite: InviteDelete,
    pub seq: ClientSeq,
}

/// multiple user or server invites were deleted
#[record]
pub struct ClientInviteDeleteBulk {
    #[serde(flatten)]
    pub invite: InviteDeleteBulk,
    pub seq: ClientSeq,
}
