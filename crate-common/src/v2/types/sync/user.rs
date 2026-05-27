#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{harvest::Harvest, preferences::PreferencesGlobal, UserId};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DispatchUser {
    pub user_id: UserId,

    // /// the channel sync sequence number of this event
    // ///
    // /// used for offline sync. only populated if this dispatch incremented the sequence number.
    // // TODO: skip serializing if none
    // seq: Option<ChannelSeq>,
    #[serde(flatten)]
    pub inner: DispatchUserInner,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DispatchUserInner {
    /// a user's harvest state was updated
    HarvestUpdate { harvest: Box<Harvest> },

    /// a user's global preferences was updated
    PreferencesGlobal { config: Box<PreferencesGlobal> },
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

    // UserCreate {
    //     user: User,
    // },

    // UserUpdate {
    //     user: User,
    // },

    // UserDelete,

    // PresenceUpdate {
    //     presence: Presence,
    // },

    // /// an interaction was created
    // ///
    // /// sent to the the user who created this and the target application
    // InteractionCreate {
    //     interaction: Box<Interaction>,

    //     user_id: UserId,

    //     /// the nonce
    //     ///
    //     /// taken from the `Ideompotency-Key` header. only sent to the user.
    //     nonce: Option<String>,
    // },

    // InteractionSuccess {
    //     user_id: UserId,
    //     interaction_id: InteractionId,
    //     nonce: Option<String>,
    // },

    // InteractionFailure {
    //     user_id: UserId,
    //     interaction_id: InteractionId,
    //     nonce: Option<String>,
    //     error_code: InteractionErrorCode,
    // },
    // // InteractionAutocompletionCreate
    // // InteractionModalCreate
}
