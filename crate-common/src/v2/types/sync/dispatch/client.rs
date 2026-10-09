use crate::v1::types::{harvest::Harvest, mirror::seq::ClientSeq, preferences::PreferencesGlobal};
use lamprey_macros::record;

/// something happened to the client user
#[record]
#[serde(tag = "type")]
pub enum DispatchClient {
    HarvestUpdate(HarvestUpdate),
    PreferencesGlobalUpdate(PreferencesGlobalUpdate),
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

impl DispatchClient {
    /// the client sync sequence number of this event
    pub fn seq(&self) -> ClientSeq {
        match self {
            DispatchClient::HarvestUpdate(a) => a.seq,
            DispatchClient::PreferencesGlobalUpdate(a) => a.seq,
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
