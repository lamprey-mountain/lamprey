use crate::{
    prelude::*,
    services::notifications::{MentionedUsers, ServiceNotifications},
};
use common::{
    v1::types::{
        Channel, ChannelId, Message, Room, RoomId, UserId,
        notifications::{
            Notification, NotificationType,
            preferences::{
                Mute, NotifsChannel, NotifsGlobal, NotifsMessages, NotifsReactions, NotifsReplies,
                NotifsRoom, NotifsThreads,
            },
        },
        util::Time,
    },
    v2::types::NotificationId,
};

/// a set of notification preferences for a user
pub struct Preferences {
    // TEMP: make fields pub
    pub global: NotifsGlobal,
    pub room: Option<NotifsRoom>,
    pub channel: Option<NotifsChannel>,
}

impl Preferences {
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

impl ServiceNotifications {
    /// load a user's notification preferences
    pub(super) async fn preferences(
        &self,
        user_id: UserId,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
    ) -> Result<Preferences> {
        let cache = &self.globals.services().cache;

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

        Ok(Preferences {
            global,
            room,
            channel,
        })
    }
}
