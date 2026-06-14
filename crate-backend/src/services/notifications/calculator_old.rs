//! notification preference calculator

use common::v1::types::notifications::preferences::{
    Mute, NotifsChannel, NotifsGlobal, NotifsMessages, NotifsReactions, NotifsReplies, NotifsRoom,
    NotifsThreads,
};
use common::v1::types::notifications::{Notification, NotificationType};
use common::v1::types::util::Time;
use common::v1::types::{Channel, ChannelType, Message, Room, UserId};

use crate::state::ServerState2;
use crate::{Result, ServerStateInner};

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

/// notification calculator
pub struct Calculator {
    /// the user's global notification preferences
    global: NotifsGlobal,

    room: Option<Room>,
    channel: Option<Channel>,
}

impl Calculator {
    /// load a user's notification preferences
    pub async fn load(
        state: &ServerStateInner,
        user_id: UserId,
        notif: &Notification,
    ) -> Result<Self> {
        let srv = state.services();
        let global = srv.cache.preferences_get(user_id).await?.notifs;

        // NOTE: why do i call .ok() instead of propagating errors?
        let channel_id = notif.channel_id();
        let channel = if let Some(id) = channel_id {
            srv.channels.get(id, Some(user_id)).await.ok()
        } else {
            None
        };

        let room_id = channel.as_ref().and_then(|ch| ch.room_id);
        let room = if let Some(id) = room_id {
            srv.rooms.get(id, Some(user_id)).await.ok()
        } else {
            None
        };

        Ok(Self {
            global,
            room,
            channel,
        })
    }

    // pub async fn load_from_message(
    //     state: &ServerState2,
    //     user_id: UserId,
    //     message: &Message,
    // ) -> Result<Self> {
    //     let srv = state.services();
    //     let global = srv.cache.preferences_get(user_id).await?.notifs;

    //     let channel_id = message.channel_id;
    //     let channel = srv.channels.get(message.channel_id, Some(user_id)).await?;

    //     let room_id = channel.as_ref().and_then(|ch| ch.room_id);
    //     let room = if let Some(id) = room_id {
    //         srv.rooms.get(id, Some(user_id)).await?
    //     } else {
    //         None
    //     };

    //     Ok(Self {
    //         global,
    //         room,
    //         channel,
    //     })
    // }

    /// check if global, room, or channel is muted
    pub fn is_muted(&self) -> bool {
        let now = Time::now_utc();
        let check_mute = |mute: &Mute| mute.expires_at.is_none() || mute.expires_at.unwrap() > now;

        if self
            .channel
            .as_ref()
            .and_then(|c| c.preferences.as_ref())
            .and_then(|p| p.notifs.mute.as_ref())
            .map_or(false, check_mute)
        {
            return true;
        }

        if self
            .room
            .as_ref()
            .and_then(|r| r.preferences.as_ref())
            .and_then(|p| p.notifs.mute.as_ref())
            .map_or(false, check_mute)
        {
            return true;
        }

        self.global.mute.as_ref().map_or(false, check_mute)
    }

    pub fn resolve_messages(&self) -> NotifsMessages {
        self.channel
            .as_ref()
            .and_then(|c| c.preferences.as_ref())
            .and_then(|p| p.notifs.messages.clone())
            .or_else(|| {
                self.room
                    .as_ref()
                    .and_then(|r| r.preferences.as_ref())
                    .and_then(|p| p.notifs.messages.clone())
            })
            .unwrap_or_else(|| self.global.messages.clone())
    }

    pub fn resolve_replies(&self) -> NotifsReplies {
        self.channel
            .as_ref()
            .and_then(|c| c.preferences.as_ref())
            .and_then(|p| p.notifs.replies.clone())
            .or_else(|| {
                self.room
                    .as_ref()
                    .and_then(|r| r.preferences.as_ref())
                    .and_then(|p| p.notifs.replies.clone())
            })
            .unwrap_or_else(|| self.global.replies.clone())
    }

    pub fn resolve_threads(&self) -> NotifsThreads {
        self.channel
            .as_ref()
            .and_then(|c| c.preferences.as_ref())
            .and_then(|p| p.notifs.threads.clone())
            .or_else(|| {
                self.room
                    .as_ref()
                    .and_then(|r| r.preferences.as_ref())
                    .and_then(|p| p.notifs.threads.clone())
            })
            .unwrap_or_else(|| self.global.threads.clone())
    }
}

/// calculate what action should be done for this notification
pub async fn calculate(
    state: &ServerStateInner,
    user_id: UserId,
    notification: &Notification,
) -> Result<NotificationAction> {
    let calc = Calculator::load(state, user_id, &notification).await?;

    let action = if calc.is_muted() {
        match notification.ty {
            // friend requests always go in the inbox
            NotificationType::FriendRequestSent { .. }
            | NotificationType::FriendRequestReceived { .. }
            | NotificationType::FriendRequestAccepted { .. } => NotificationAction::Inbox,

            // everything else gets dropped
            _ => NotificationAction::Skip,
        }
    } else {
        match notification.ty {
            NotificationType::Message {
                mention_user,
                mention_everyone,
                mention_role,
                reply,
                ..
            } => {
                // resolve actions for replies
                let reply_action = if reply {
                    calc.resolve_replies().into()
                } else {
                    NotificationAction::Skip
                };

                // room notification preferences restrict what counts as a mention
                let mentioned = if let Some(room_prefs) = calc
                    .room
                    .as_ref()
                    .and_then(|r| r.preferences.as_ref())
                    .map(|p| &p.notifs)
                {
                    let mention_role = mention_role && room_prefs.mention_roles;
                    let mention_everyone = mention_everyone && room_prefs.mention_everyone;
                    mention_user || mention_role || mention_everyone
                } else {
                    mention_user || mention_role || mention_everyone
                };

                // resolve actions for messages
                let prefs = calc.resolve_messages();
                let message_action = match prefs {
                    NotifsMessages::Everything => NotificationAction::Push,
                    NotifsMessages::Watching => {
                        if mentioned {
                            NotificationAction::Push
                        } else {
                            NotificationAction::Inbox
                        }
                    }
                    NotifsMessages::Mentions => {
                        if mentioned {
                            NotificationAction::Push
                        } else {
                            NotificationAction::Skip
                        }
                    }
                    NotifsMessages::Nothing => NotificationAction::Skip,
                };

                reply_action.merge(message_action)
            }
            NotificationType::Thread { .. } => calc.resolve_threads().into(),
            NotificationType::Reaction { .. } => match calc.global.reactions {
                NotifsReactions::Always => NotificationAction::Push,
                NotifsReactions::Restricted => {
                    todo!("is dm or private room")
                }
                NotifsReactions::Dms => {
                    let chan = calc.channel.unwrap();
                    if matches!(chan.ty, ChannelType::Dm | ChannelType::Gdm) {
                        NotificationAction::Push
                    } else {
                        NotificationAction::Skip
                    }
                }
                NotifsReactions::Nothing => NotificationAction::Skip,
            },
            NotificationType::FriendRequestSent { .. }
            | NotificationType::FriendRequestReceived { .. }
            | NotificationType::FriendRequestAccepted { .. } => NotificationAction::Push,
        }
    };

    Ok(action)
}

impl NotificationAction {
    /// Merge these actions with other actions
    pub fn merge(&self, other: Self) -> Self {
        use NotificationAction::*;
        match (*self, other) {
            (Push, _) | (_, Push) => Push,
            (Inbox, _) | (_, Inbox) => Inbox,
            (Skip, Skip) => Skip,
        }
    }

    /// Whether this notification should be sent as a push notification
    pub fn should_push(&self) -> bool {
        matches!(self, NotificationAction::Push)
    }

    /// Whether this notification should be added to the inbox
    pub fn should_add_to_inbox(&self) -> bool {
        matches!(self, NotificationAction::Inbox | NotificationAction::Push)
    }
}

impl From<NotifsMessages> for NotificationAction {
    fn from(value: NotifsMessages) -> Self {
        match value {
            NotifsMessages::Nothing => NotificationAction::Skip,
            NotifsMessages::Mentions => NotificationAction::Inbox,
            NotifsMessages::Watching => NotificationAction::Inbox,
            NotifsMessages::Everything => NotificationAction::Push,
        }
    }
}

impl From<NotifsReplies> for NotificationAction {
    fn from(value: NotifsReplies) -> Self {
        match value {
            NotifsReplies::Notify => NotificationAction::Push,
            NotifsReplies::Watching => NotificationAction::Inbox,
            NotifsReplies::Nothing => NotificationAction::Skip,
        }
    }
}

impl From<NotifsThreads> for NotificationAction {
    fn from(value: NotifsThreads) -> Self {
        match value {
            NotifsThreads::Notify => NotificationAction::Push,
            NotifsThreads::Inbox => NotificationAction::Inbox,
            NotifsThreads::Nothing => NotificationAction::Skip,
        }
    }
}
