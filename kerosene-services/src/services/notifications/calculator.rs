use common::{
    v1::types::{
        notifications::preferences::{Mute, NotifsMessages, NotifsReplies, NotifsThreads},
        preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom},
        util::Time,
    },
    v2::types::{ChannelId, RoomId, UserId},
};

use crate::prelude::*;

/// a set of notification preferences for a user
pub struct Preferences {
    global: Arc<PreferencesGlobal>,
    room: Option<Arc<PreferencesRoom>>,
    channel: Option<Arc<PreferencesChannel>>,
}

/// actions to take on this event
#[derive(Debug, Default)]
pub struct Actions {
    push: bool,
    inbox: bool,
    bump_mention_count: bool,
}

/// notification calculator
pub struct Calculator {
    _globals: Globals,
    // // context
    // _room: Option<Room>,
    // _channel: Option<Channel>,
    // _message: Option<Message>,
    // // notification: Option<Notification>,
    // // TODO
}

impl Preferences {
    /// load a user's notification preferences
    pub async fn load(
        globals: &Globals,
        user_id: UserId,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
    ) -> Result<Self> {
        let srv = &globals.services();
        let global = srv.preferences.get_global(user_id).await?;

        let room = if let Some(id) = room_id {
            srv.preferences.get_room(user_id, id).await.ok()
        } else {
            None
        };

        let channel = if let Some(id) = channel_id {
            srv.preferences.get_channel(user_id, id).await.ok()
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
            .and_then(|c| c.notifs.mute.as_ref())
            .map_or(false, check_mute)
        {
            return true;
        }

        if self
            .room
            .as_ref()
            .and_then(|r| r.notifs.mute.as_ref())
            .map_or(false, check_mute)
        {
            return true;
        }

        self.global.notifs.mute.as_ref().map_or(false, check_mute)
    }

    pub fn resolve_messages(&self) -> &NotifsMessages {
        self.channel
            .as_ref()
            .and_then(|c| c.notifs.messages.as_ref())
            .or_else(|| self.room.as_ref().and_then(|r| r.notifs.messages.as_ref()))
            .unwrap_or(&self.global.notifs.messages)
    }

    pub fn resolve_replies(&self) -> &NotifsReplies {
        self.channel
            .as_ref()
            .and_then(|c| c.notifs.replies.as_ref())
            .or_else(|| self.room.as_ref().and_then(|r| r.notifs.replies.as_ref()))
            .unwrap_or(&self.global.notifs.replies)
    }

    pub fn resolve_threads(&self) -> &NotifsThreads {
        self.channel
            .as_ref()
            .and_then(|c| c.notifs.threads.as_ref())
            .or_else(|| self.room.as_ref().and_then(|r| r.notifs.threads.as_ref()))
            .unwrap_or(&self.global.notifs.threads)
    }
}

impl Actions {
    /// Merge these actions with other actions
    pub fn merge(&mut self, other: Self) {
        self.push |= other.push;
        self.inbox |= other.inbox;
        self.bump_mention_count |= other.bump_mention_count;
    }

    /// Whether this notification should be sent as a push notification
    pub fn should_push(&self) -> bool {
        self.push
    }

    /// Whether this notification should be added to the inbox
    pub fn should_add_to_inbox(&self) -> bool {
        self.inbox
    }

    /// Whether the mention count should be incremented
    pub fn should_increment_mention_count(&self) -> bool {
        self.bump_mention_count
    }
}

// TODO: implement or remove?
// impl Calculator {
//     pub async fn load_for_message(
//         _state: Globals,
//         _channel: &Channel,
//         _message: &Message,
//     ) -> Result<Self> {
//         todo!()
//     }

//     pub async fn load_for_notification(_state: Globals, _notif: &Notification) -> Result<Self> {
//         todo!()
//     }

//     /// calculate notification actions for a user
//     pub async fn calculate(&self, _user_id: UserId) -> Result<Actions> {
//         // TODO: return a Notification/NotificationType?
//         todo!()
//     }
// }
