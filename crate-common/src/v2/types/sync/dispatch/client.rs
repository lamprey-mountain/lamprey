use crate::v1::types::{
    harvest::Harvest, mirror::seq::ClientSeq, misc::Time, preferences::PreferencesGlobal,
};
use crate::v2::types::{ChannelId, MessageId};
use lamprey_macros::record;

/// something happened to the client user
#[record]
#[serde(tag = "type")]
pub enum DispatchClient {
    HarvestUpdate(HarvestUpdate),
    PreferencesGlobalUpdate(PreferencesGlobalUpdate),
    ReadStateUpdate(ReadStateUpdate),
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
            DispatchClient::ReadStateUpdate(a) => a.seq,
        }
    }
}

// PreferencesRoom {
//     room_id: RoomId,
//     config: PreferencesRoom,
// },

// PreferencesChannel {
//     channel_id: ChannelId,
//     config: PreferencesChannel,
// },

// PreferencesUser {
//     target_user_id: UserId,
//     config: PreferencesUser,
// },

// SessionCreate {
//     session: Box<Session>,
// },

// SessionUpdate {
//     session: Box<Session>,
// },

// SessionDelete {
//     id: SessionId,
//     user_id: Option<UserId>,
// },

// SessionDeleteAll {
//     user_id: UserId,
// },

// RelationshipUpsert {
//     user_id: UserId,
//     target_user_id: UserId,
//     relationship: Relationship,
// },

// RelationshipDelete {
//     user_id: UserId,
//     target_user_id: UserId,
// },

// ConnectionCreate {
//     user_id: UserId,
//     connection: Connection,
// },

// ConnectionDelete {
//     user_id: UserId,
//     app_id: ApplicationId,
// },

// InboxNotificationCreate {
//     user_id: UserId,
//     notification: Notification,
// },

// InboxMarkRead {
//     user_id: UserId,
//     #[cfg_attr(feature = "serde", serde(flatten))]
//     params: NotificationMarkRead,
// },

// InboxMarkUnread {
//     user_id: UserId,
//     #[cfg_attr(feature = "serde", serde(flatten))]
//     params: NotificationMarkRead,
// },

// InboxFlush {
//     user_id: UserId,
//     #[cfg_attr(feature = "serde", serde(flatten))]
//     params: NotificationFlush,
// },
