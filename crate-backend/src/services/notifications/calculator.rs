use crate::{prelude::*, services::notifications::ServiceNotifications};
use common::v1::types::{
    Channel, ChannelId, Message, Room, RoomId, UserId,
    notifications::{
        Notification,
        preferences::{
            Mute, NotifsChannel, NotifsGlobal, NotifsMessages, NotifsReplies, NotifsRoom,
            NotifsThreads,
        },
    },
    util::Time,
};

/// a set of notification preferences for a user
pub struct Preferences {
    global: NotifsGlobal,
    room: Option<NotifsRoom>,
    channel: Option<NotifsChannel>,
}

/// notification calculator
pub struct Calculator {
    _state: Globals,

    // context
    _room: Option<Room>,
    _channel: Option<Channel>,
    _message: Option<Message>,
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
        _state: Globals,
        _channel: &Channel,
        _message: &Message,
    ) -> Result<Self> {
        todo!()
    }

    pub async fn load_for_notification(_state: Globals, _notif: &Notification) -> Result<Self> {
        todo!()
    }

    /// calculate notification actions for a user
    pub async fn calculate(&self, _user_id: UserId) -> Result<Actions> {
        // TODO: return a Notification/NotificationType?
        todo!()
    }
}

impl Preferences {
    /// load a user's notification preferences
    pub async fn load(
        state: &Globals,
        user_id: UserId,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
    ) -> Result<Self> {
        let cache = &state.services().cache;
        let global = cache.preferences_get(user_id).await?.notifs;

        let room = if let Some(id) = room_id {
            cache
                .preferences_room_get(user_id, id)
                .await
                .ok()
                .map(|p| p.notifs)
        } else {
            None
        };

        let channel = if let Some(id) = channel_id {
            cache
                .preferences_channel_get(user_id, id)
                .await
                .ok()
                .map(|p| p.notifs)
        } else {
            None
        };

        Ok(Self {
            global,
            room,
            channel,
        })
    }

    /// check if global, room, or channel is muted
    pub fn is_muted(&self) -> bool {
        let now = Time::now_utc();
        let check_mute = |mute: &Mute| {
            mute.expires_at
                .as_ref()
                .map_or(true, |&expires| expires > now)
        };

        if self
            .channel
            .as_ref()
            .and_then(|c| c.mute.as_ref())
            .map_or(false, check_mute)
        {
            return true;
        }

        if self
            .room
            .as_ref()
            .and_then(|r| r.mute.as_ref())
            .map_or(false, check_mute)
        {
            return true;
        }

        self.global.mute.as_ref().map_or(false, check_mute)
    }

    pub fn resolve_messages(&self) -> &NotifsMessages {
        self.channel
            .as_ref()
            .and_then(|c| c.messages.as_ref())
            .or_else(|| self.room.as_ref().and_then(|r| r.messages.as_ref()))
            .unwrap_or(&self.global.messages)
    }

    pub fn resolve_replies(&self) -> &NotifsReplies {
        self.channel
            .as_ref()
            .and_then(|c| c.replies.as_ref())
            .or_else(|| self.room.as_ref().and_then(|r| r.replies.as_ref()))
            .unwrap_or(&self.global.replies)
    }

    pub fn resolve_threads(&self) -> &NotifsThreads {
        self.channel
            .as_ref()
            .and_then(|c| c.threads.as_ref())
            .or_else(|| self.room.as_ref().and_then(|r| r.threads.as_ref()))
            .unwrap_or(&self.global.threads)
    }
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
