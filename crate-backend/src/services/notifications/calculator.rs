// TODO: design this better

use crate::{prelude::*, services::notifications::ServiceNotifications};
use common::v1::types::{
    Channel, Message, Room, UserId,
    notifications::{
        Notification,
        preferences::{NotifsChannel, NotifsGlobal, NotifsRoom},
    },
};

/// a set of notification preferences for a user
pub struct Preferences {
    global: NotifsGlobal,
    room: Option<NotifsRoom>,
    channel: Option<NotifsChannel>,
}

/// notification calculator
pub struct Calculator {
    state: ServerState2,

    // context
    room: Option<Room>,
    channel: Option<Channel>,
    message: Option<Message>,
    // notification: Option<Notification>,
    // TODO
}

/// actions to take on this event
pub struct Actions {
    // TODO
}

impl Calculator {
    // pub async fn load_for_message(state: ServerState2, message: &Message) -> Result<Self> {
    pub async fn load_for_message(
        state: ServerState2,
        channel: &Channel,
        message: &Message,
    ) -> Result<Self> {
        todo!()
    }

    pub async fn load_for_notification(state: ServerState2, notif: &Notification) -> Result<Self> {
        todo!()
    }

    /// calculate notification actions for a user
    pub async fn calculate(&self, user_id: UserId) -> Result<Actions> {
        // TODO: return a Notification/NotificationType?
        todo!()
    }
}

impl Preferences {
    /// load a user's notification preferences
    pub async fn load(state: &ServerState2, user_id: UserId) -> Result<Self> {
        todo!()
    }

    /// check if global, room, or channel is muted
    pub fn is_muted(&self) -> bool {
        todo!()
    }

    // TODO
    // pub fn resolve_messages(&self) -> NotifsMessages {
    // pub fn resolve_replies(&self) -> NotifsReplies {
    // pub fn resolve_threads(&self) -> NotifsThreads {
}

impl Actions {
    /// Whether this notification should be sent as a push notification
    pub fn should_push(&self) -> bool {
        todo!()
    }

    /// Whether this notification should be added to the inbox
    pub fn should_add_to_inbox(&self) -> bool {
        todo!()
    }

    /// Whether the mention count should be incremented
    pub fn should_increment_mention_count(&self) -> bool {
        todo!()
    }
}

impl ServiceNotifications {
    // /// calculate the actions to take for a notification
    // pub async fn calculate_actions(
    //     &self,
    //     user_id: UserId,
    //     notif: &Notification,
    // ) -> Result<NotificationAction> {
    //     calculate(&self.state, user_id, notif).await
    // }
}
