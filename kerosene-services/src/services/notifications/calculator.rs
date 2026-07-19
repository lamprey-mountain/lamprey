use crate::{prelude::*, services::notifications::MentionedUsers};
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

/// actions to take on this event
pub struct Actions {
    should_push: bool,
    should_add_to_inbox: bool,
    should_increment_mention_count: bool,
    notification: Option<Notification>,
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

impl Actions {
    /// Whether this notification should be sent as a push notification
    pub fn should_push(&self) -> bool {
        self.should_push
    }

    /// Whether this notification should be added to the inbox
    pub fn should_add_to_inbox(&self) -> bool {
        self.should_add_to_inbox
    }

    /// Whether the mention count should be incremented
    pub fn should_increment_mention_count(&self) -> bool {
        self.should_increment_mention_count
    }

    /// Get the notification that should be created for this user
    pub fn notification(&self) -> Option<&Notification> {
        self.notification.as_ref()
    }
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

/// a set of notification preferences for a user
pub struct Preferences {
    global: NotifsGlobal,
    room: Option<NotifsRoom>,
    channel: Option<NotifsChannel>,
}

/// notification calculator
pub struct Calculator {
    globals: Globals,

    // context
    room: Option<Room>,
    channel: Option<Channel>,
    replied_message: Option<Message>,
    message: Option<Message>,
    mentioned_users: Option<MentionedUsers>,
    notification: Option<Notification>,
}

impl Calculator {
    pub async fn load_for_message(
        globals: Globals,
        channel: &Channel,
        message: &Message,
    ) -> Result<Self> {
        let srv = globals.services();

        let replied_message = if let Some(reply_id) = message.latest_version.reply_id {
            srv.messages.get(channel.id, reply_id, None).await.ok()
        } else {
            None
        };

        let room = if let Some(room_id) = channel.room_id {
            srv.rooms.get(room_id, None).await.ok()
        } else {
            None
        };

        let mentioned_users = srv
            .notifications
            .get_mentioned_users(channel, message)
            .await
            .ok();

        Ok(Self {
            globals,
            room,
            channel: Some(channel.clone()),
            replied_message,
            message: Some(message.clone()),
            mentioned_users,
            notification: None,
        })
    }

    pub async fn load_for_notification(globals: Globals, notif: &Notification) -> Result<Self> {
        let srv = globals.services();

        let channel_id = notif.channel_id();
        let channel = if let Some(id) = channel_id {
            srv.channels.get(id, None).await.ok()
        } else {
            None
        };

        let room_id = channel.as_ref().and_then(|ch| ch.room_id);
        let room = if let Some(id) = room_id {
            srv.rooms.get(id, None).await.ok()
        } else {
            None
        };

        Ok(Self {
            globals,
            room,
            channel,
            replied_message: None,
            message: None,
            mentioned_users: None,
            notification: Some(notif.clone()),
        })
    }

    /// calculate notification actions for a user
    // TODO: drop notification if message author is ignored or blocked
    pub async fn calculate(&self, user_id: UserId) -> Result<Actions> {
        let room_id = self.channel.as_ref().and_then(|c| c.room_id);
        let channel_id = self.channel.as_ref().map(|c| c.id);
        let prefs = Preferences::load(&self.globals, user_id, room_id, channel_id).await?;

        // NOTE: maybe make this an enum
        let (notif, action) = if let Some(message) = &self.message {
            let channel = self.channel.as_ref().ok_or_else(|| {
                Error::Internal("missing channel for message notification".to_string())
            })?;

            let mention_user = self
                .mentioned_users
                .as_ref()
                .map_or(false, |m| m.users_from_direct.contains(&user_id));
            let mention_everyone = self
                .mentioned_users
                .as_ref()
                .map_or(false, |m| m.users_from_everyone.contains(&user_id));
            let mention_role = self
                .mentioned_users
                .as_ref()
                .map_or(false, |m| m.users_from_role.contains(&user_id));
            let reply = self
                .replied_message
                .as_ref()
                .map_or(false, |m| m.author_id == user_id);

            let notif = Notification {
                id: NotificationId::new(),
                ty: NotificationType::Message {
                    room_id: channel.room_id,
                    channel_id: channel.id,
                    message_id: message.id,
                    user_id: message.author_id,
                    mention_user,
                    mention_everyone,
                    mention_role,
                    reply,
                },
                added_at: Time::now_utc(),
                read_at: None,
                note: None,
            };

            let action = self.calculate_message_action(
                &prefs,
                mention_user,
                mention_everyone,
                mention_role,
                reply,
            );
            (Some(notif), action)
        } else if let Some(notif) = &self.notification {
            let action = self.calculate_notification_action(&prefs, notif);
            (Some(notif.clone()), action)
        } else {
            return Err(Error::Internal(
                "no message or notification context in calculator".to_string(),
            ));
        };

        // Determine if the user was actively mentioned after applying room rules
        let mentioned = if let Some(n) = &notif {
            if let NotificationType::Message {
                mention_user,
                mention_everyone,
                mention_role,
                ..
            } = &n.ty
            {
                let room_prefs = prefs.room.as_ref();
                let mention_role_allowed = if let Some(rp) = room_prefs {
                    *mention_role && rp.mention_roles
                } else {
                    *mention_role
                };
                let mention_everyone_allowed = if let Some(rp) = room_prefs {
                    *mention_everyone && rp.mention_everyone
                } else {
                    *mention_everyone
                };
                *mention_user || mention_role_allowed || mention_everyone_allowed
            } else {
                false
            }
        } else {
            false
        };

        let is_dm = self.channel.as_ref().map_or(false, |c| c.ty.is_dm());
        let should_increment_mention_count = !prefs.is_muted() && (mentioned || is_dm);

        Ok(Actions {
            should_push: action.should_push(),
            should_add_to_inbox: action.should_add_to_inbox(),
            should_increment_mention_count,
            notification: notif,
        })
    }

    fn calculate_message_action(
        &self,
        prefs: &Preferences,
        mention_user: bool,
        mention_everyone: bool,
        mention_role: bool,
        reply: bool,
    ) -> NotificationAction {
        if prefs.is_muted() {
            return NotificationAction::Skip;
        }

        let reply_action = if reply {
            prefs.resolve_replies().clone().into()
        } else {
            NotificationAction::Skip
        };

        let mentioned = if let Some(room_prefs) = &prefs.room {
            let mention_role_allowed = mention_role && room_prefs.mention_roles;
            let mention_everyone_allowed = mention_everyone && room_prefs.mention_everyone;
            mention_user || mention_role_allowed || mention_everyone_allowed
        } else {
            mention_user || mention_role || mention_everyone
        };

        let msg_pref = prefs.resolve_messages();
        let message_action = match msg_pref {
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

    fn calculate_notification_action(
        &self,
        prefs: &Preferences,
        notif: &Notification,
    ) -> NotificationAction {
        if prefs.is_muted() {
            return match notif.ty {
                NotificationType::FriendRequestSent { .. }
                | NotificationType::FriendRequestReceived { .. }
                | NotificationType::FriendRequestAccepted { .. } => NotificationAction::Inbox,
                _ => NotificationAction::Skip,
            };
        }

        match &notif.ty {
            NotificationType::Message {
                mention_user,
                mention_everyone,
                mention_role,
                reply,
                ..
            } => self.calculate_message_action(
                prefs,
                *mention_user,
                *mention_everyone,
                *mention_role,
                *reply,
            ),
            NotificationType::Thread { .. } => prefs.resolve_threads().clone().into(),
            NotificationType::Reaction { .. } => match prefs.global.reactions {
                NotifsReactions::Always => NotificationAction::Push,
                // FIXME: NotifsReactions::Restricted should be enabled for private rooms?
                // i may remove Restricted soon
                NotifsReactions::Restricted | NotifsReactions::Dms => {
                    if let Some(chan) = &self.channel {
                        if chan.ty.is_dm() {
                            NotificationAction::Push
                        } else {
                            NotificationAction::Skip
                        }
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
    }

    // TODO: fn room_id(&self) -> Option<RoomId>
    // TODO: fn channel_id(&self) -> Option<ChannelId>
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
