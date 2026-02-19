//! notification preference calculator

use common::v1::types::notifications::preferences::NotifsMessages;
use common::v1::types::notifications::{Notification, NotificationType};
use common::v1::types::util::Time;
use common::v1::types::{Channel, ChannelType, Message, Room, UserId};

use crate::{Result, ServerStateInner};

pub struct NotificationActionCalculator {
    state: std::sync::Arc<ServerStateInner>,
    user_id: UserId,
    notification: Notification,
    channel: Option<Channel>,
    room: Option<Room>,
    message: Option<Message>,
    mentions_user: bool,
    mentions_role: bool,
    mentions_everyone: bool,
}

/// What action to take for a notification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationAction {
    /// Don't do anything
    Skip,

    /// Add to inbox only, no push notification
    Inbox,

    /// Send push notification and add to inbox
    Push,
}

impl NotificationActionCalculator {
    pub fn new(
        state: std::sync::Arc<ServerStateInner>,
        user_id: UserId,
        notification: Notification,
    ) -> Self {
        NotificationActionCalculator {
            state,
            user_id,
            notification,
            channel: None,
            room: None,
            message: None,
            mentions_user: false,
            mentions_role: false,
            mentions_everyone: false,
        }
    }

    /// this notification occured in a dm/gdm channel
    pub fn in_dm(self) -> Self {
        self
    }

    /// this notification occured in a private room
    pub fn in_private_room(self) -> Self {
        self
    }

    /// this notification occured in a public room
    pub fn in_public_room(self) -> Self {
        self
    }

    /// this notification mentions the user
    pub fn mentions_user(mut self) -> Self {
        self.mentions_user = true;
        self
    }

    /// this notification mentions a role the user has
    pub fn mentions_role(mut self) -> Self {
        self.mentions_role = true;
        self
    }

    /// this notification mentions everyone in a channel
    pub fn mentions_everyone(mut self) -> Self {
        self.mentions_everyone = true;
        self
    }

    pub async fn action(self) -> Result<NotificationAction> {
        // Fetch channel
        let channel_id = match self.notification.channel_id() {
            Some(id) => id,
            None => return Ok(NotificationAction::Skip),
        };

        let channel = self.state.data().channel_get(channel_id).await.ok();

        // Fetch room if channel has room_id
        let room = if let Some(ref ch) = channel {
            if let Some(room_id) = ch.room_id {
                self.state.data().room_get(room_id).await.ok()
            } else {
                None
            }
        } else {
            None
        };

        // Fetch message if this is a message notification
        let message = match &self.notification.ty {
            NotificationType::Message {
                channel_id,
                message_id,
                ..
            }
            | NotificationType::Reaction {
                channel_id,
                message_id,
                ..
            } => self
                .state
                .data()
                .message_get(*channel_id, *message_id, self.user_id)
                .await
                .ok(),
        };

        // Check if this is a DM/GDM
        let is_dm = channel
            .as_ref()
            .map(|ch| matches!(ch.ty, ChannelType::Dm | ChannelType::Gdm))
            .unwrap_or(false);

        // Check if this is a private room
        let is_private_room = room.as_ref().map(|r| !r.public).unwrap_or(false);

        // Check if this is a public room
        let is_public_room = room.as_ref().map(|r| r.public).unwrap_or(false);

        // Check mentions from message
        let message_mentions_user = message
            .as_ref()
            .map(|m| {
                m.latest_version
                    .mentions
                    .users
                    .iter()
                    .any(|u| u.id == self.user_id)
            })
            .unwrap_or(false);

        let message_mentions_everyone = message
            .as_ref()
            .map(|m| m.latest_version.mentions.everyone)
            .unwrap_or(false);

        // For role mentions, we'd need to check user's roles vs mentioned roles
        // This is a simplified check - in production you'd fetch user's roles
        let message_mentions_role = message
            .as_ref()
            .map(|m| !m.latest_version.mentions.roles.is_empty())
            .unwrap_or(false);

        let srv = self.state.services();

        // Load channel config
        let channel_config = if let Some(ref ch) = channel {
            srv.cache
                .user_config_channel_get(self.user_id, ch.id)
                .await
                .ok()
                .map(|c| c.notifs)
        } else {
            None
        };

        // Load room config
        let room_config = if let Some(ref r) = room {
            srv.cache
                .user_config_room_get(self.user_id, r.id)
                .await
                .ok()
                .map(|c| c.notifs)
        } else {
            None
        };

        // Load global config
        let global_config = srv.cache.user_config_get(self.user_id).await?.notifs;

        // Check channel-level mute first (highest priority)
        if let Some(ref channel_config) = channel_config {
            if let Some(ref mute) = channel_config.mute {
                if mute.expires_at.is_none() || mute.expires_at.unwrap() > Time::now_utc() {
                    return Ok(NotificationAction::Skip);
                }
            }

            if let Some(ref messages_config) = channel_config.messages {
                return Ok(self.evaluate_messages_config(
                    messages_config,
                    is_dm,
                    is_private_room,
                    is_public_room,
                    message_mentions_user,
                    message_mentions_role,
                    message_mentions_everyone,
                ));
            }
        }

        // Check room-level config
        if let Some(ref room_config) = room_config {
            if let Some(ref mute) = room_config.mute {
                if mute.expires_at.is_none() || mute.expires_at.unwrap() > Time::now_utc() {
                    return Ok(NotificationAction::Skip);
                }
            }

            // Check room-level messages setting
            if let Some(ref messages_config) = room_config.messages {
                return Ok(self.evaluate_messages_config(
                    messages_config,
                    is_dm,
                    is_private_room,
                    is_public_room,
                    message_mentions_user,
                    message_mentions_role,
                    message_mentions_everyone,
                ));
            }

            // Check room-level mention settings
            if is_public_room {
                // Check @everyone mentions
                if message_mentions_everyone && !room_config.mention_everyone {
                    return Ok(NotificationAction::Skip);
                }

                // Check @role mentions
                if message_mentions_role && !room_config.mention_roles {
                    return Ok(NotificationAction::Skip);
                }
            }
        }

        // Check global mute
        if let Some(ref mute) = global_config.mute {
            if mute.expires_at.is_none() || mute.expires_at.unwrap() > Time::now_utc() {
                return Ok(NotificationAction::Skip);
            }
        }

        // Fall back to global messages setting
        Ok(self.evaluate_messages_config(
            &global_config.messages,
            is_dm,
            is_private_room,
            is_public_room,
            message_mentions_user,
            message_mentions_role,
            message_mentions_everyone,
        ))
    }

    fn evaluate_messages_config(
        &self,
        messages_config: &NotifsMessages,
        is_dm: bool,
        is_private_room: bool,
        _is_public_room: bool,
        message_mentions_user: bool,
        message_mentions_role: bool,
        message_mentions_everyone: bool,
    ) -> NotificationAction {
        match messages_config {
            NotifsMessages::Nothing => NotificationAction::Skip,
            NotifsMessages::Mentions => {
                // For Mentions mode, check if any relevant mentions occurred
                let has_relevant_mention = message_mentions_user
                    || message_mentions_role
                    || message_mentions_everyone
                    || is_dm
                    || is_private_room;

                if has_relevant_mention {
                    NotificationAction::Push
                } else {
                    NotificationAction::Inbox
                }
            }
            NotifsMessages::Watching => NotificationAction::Inbox,
            NotifsMessages::Everything => NotificationAction::Push,
        }
    }
}

impl NotificationAction {
    /// Whether this notification should be sent as a push notification
    pub fn should_push(&self) -> bool {
        matches!(self, NotificationAction::Push)
    }

    /// Whether this notification should be added to the inbox
    pub fn should_add_to_inbox(&self) -> bool {
        matches!(self, NotificationAction::Inbox | NotificationAction::Push)
    }
}

// /// Determine the appropriate action for a notification based on user preferences
// pub async fn notification_action(
//     state: &ServerStateInner,
//     user_id: UserId,
//     notif: &Notification,
// ) -> Result<NotificationAction> {
//     let srv = state.services();

//     let channel_config = srv
//         .cache
//         .user_config_channel_get(user_id, notif.channel_id)
//         .await
//         .ok();

//     if let Some(ref config) = channel_config {
//         if let Some(ref mute) = config.notifs.mute {
//             if mute.expires_at.is_none() || mute.expires_at.unwrap() > Time::now_utc() {
//                 return Ok(NotificationAction::Skip);
//             }
//         }

//         if let Some(ref messages_config) = config.notifs.messages {
//             return Ok(match messages_config {
//                 NotifsMessages::Nothing => NotificationAction::Skip,
//                 NotifsMessages::Mentions => NotificationAction::Inbox,
//                 NotifsMessages::Watching => NotificationAction::Push,
//                 NotifsMessages::Everything => NotificationAction::Push,
//             });
//         }
//     }

//     // If room_id is provided, check room preferences
//     // Note: we need to fetch the channel to get its room_id
//     let room_id = state
//         .data()
//         .channel_get(notif.channel_id)
//         .await
//         .ok()
//         .and_then(|ch| ch.room_id);

//     if let Some(room_id) = room_id {
//         let room_config = srv.cache.user_config_room_get(user_id, room_id).await.ok();

//         // Check room-level mute
//         if let Some(ref config) = room_config {
//             if let Some(ref mute) = config.notifs.mute {
//                 if mute.expires_at.is_none() || mute.expires_at.unwrap() > Time::now_utc() {
//                     return Ok(NotificationAction::Skip); // Muted forever or until a future time
//                 }
//             }

//             // Check room-level messages setting
//             if let Some(ref messages_config) = config.notifs.messages {
//                 return Ok(match messages_config {
//                     NotifsMessages::Nothing => NotificationAction::Skip,
//                     NotifsMessages::Mentions => NotificationAction::Inbox,
//                     NotifsMessages::Watching => NotificationAction::Push,
//                     NotifsMessages::Everything => NotificationAction::Push,
//                 });
//             }
//         }
//     }

//     // Fall back to global preferences
//     let global_config = srv.cache.user_config_get(user_id).await?;

//     // Check global mute
//     if let Some(ref mute) = global_config.notifs.mute {
//         if mute.expires_at.is_none() || mute.expires_at.unwrap() > Time::now_utc() {
//             return Ok(NotificationAction::Skip); // Muted forever or until a future time
//         }
//     }

//     // Check global messages setting
//     Ok(match global_config.notifs.messages {
//         NotifsMessages::Nothing => NotificationAction::Skip,
//         NotifsMessages::Mentions => NotificationAction::Inbox,
//         NotifsMessages::Watching => NotificationAction::Push,
//         NotifsMessages::Everything => NotificationAction::Push,
//     })
// }
