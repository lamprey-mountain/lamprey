use crate::{
    prelude::*,
    services::notifications::{MentionedUsers, ServiceNotifications, preferences::Preferences},
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

pub struct Actions_ {
    // PERF: use bitflags
    should_push: bool,
    should_add_to_inbox: bool,
    should_increment_mention_count: bool,
    should_add_to_thread: bool,
    notification: Option<Notification>,
}

/// Set of actions to take on an event
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Actions {
    // PERF: use bitflags
    should_push: bool,
    should_add_to_inbox: bool,
    should_increment_mention_count: bool,
    should_add_to_thread: bool,
}

// bitflags::bitflags! {
//     struct ActionsInner: u8 {
//         Push,
//         Inbox,
//         IncrementMentions,
//         IncrementUnreads,
//         AddToThread,
//     }
// }

/// Notification action calculator for an event
///
/// Calculates what actions should be done for a user.
pub struct Calculator<'a> {
    globals: Globals,
    context: CalculatorContext<'a>,
}

/// Notification calculator context for a message
// TODO: handle other context types?
pub(super) struct CalculatorContext<'a> {
    pub room: Option<&'a Room>,
    pub channel: &'a Channel,
    pub replied_message: Option<&'a Message>,
    pub message: &'a Message,
    pub mentioned_users: &'a MentionedUsers,
}

impl Actions {
    /// Don't do anything
    pub fn skip() -> Self {
        Self::default()
    }

    /// Add to inbox only, no push notification
    pub fn inbox() -> Self {
        todo!()
    }

    /// Send push notification and add to inbox
    pub fn push() -> Self {
        todo!()
    }

    /// Whether this notification should be sent as a push notification
    #[inline]
    pub fn should_push(&self) -> bool {
        self.should_push
    }

    /// Whether this notification should be added to the inbox
    #[inline]
    pub fn should_add_to_inbox(&self) -> bool {
        self.should_add_to_inbox
    }

    /// Whether the mention count should be incremented
    #[inline]
    pub fn should_increment_mention_count(&self) -> bool {
        self.should_increment_mention_count
    }

    /// Whether the user should be added to the thread
    #[inline]
    pub fn should_add_to_thread(&self) -> bool {
        self.should_add_to_thread
    }

    /// Merge these actions with other actions
    pub fn merge(&self, other: Self) -> Self {
        let should_push = self.should_push || other.should_push;
        let should_add_to_inbox =
            should_push || self.should_add_to_inbox || other.should_add_to_inbox;
        let should_increment_mention_count =
            self.should_increment_mention_count || other.should_increment_mention_count;
        let should_add_to_thread = self.should_add_to_thread || other.should_add_to_thread;

        Self {
            should_push,
            should_add_to_inbox,
            should_increment_mention_count,
            should_add_to_thread,
        }
    }

    /// Get the notification that should be created for this user
    #[inline]
    pub fn notification(&self) -> Option<&Notification> {
        // self.notification.as_ref()
        todo!()
    }
}

impl From<NotifsMessages> for Actions {
    fn from(value: NotifsMessages) -> Self {
        match value {
            NotifsMessages::Nothing => Actions::skip(),
            NotifsMessages::Mentions => Actions::inbox(),
            NotifsMessages::Watching => Actions::inbox(),
            NotifsMessages::Everything => Actions::push(),
        }
    }
}

impl From<NotifsReplies> for Actions {
    fn from(value: NotifsReplies) -> Self {
        match value {
            NotifsReplies::Notify => Actions::push(),
            NotifsReplies::Watching => Actions::inbox(),
            NotifsReplies::Nothing => Actions::skip(),
        }
    }
}

impl From<NotifsThreads> for Actions {
    fn from(value: NotifsThreads) -> Self {
        match value {
            NotifsThreads::Notify => Actions::push(),
            NotifsThreads::Inbox => Actions::inbox(),
            NotifsThreads::Nothing => Actions::skip(),
        }
    }
}

impl Calculator<'_> {
    /// calculate notification actions for a user
    // TODO: drop notification if message author is ignored or blocked
    pub async fn calculate(&self, user_id: UserId) -> Result<Actions> {
        let room_id = self.channel.as_ref().and_then(|c| c.room_id);
        let channel_id = self.channel.as_ref().map(|c| c.id);
        let prefs = self
            .globals
            .services()
            .notifications
            .preferences(user_id, room_id, channel_id)
            .await?;

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
        let is_thread = self.channel.as_ref().map_or(false, |c| c.ty.is_thread());
        let should_increment_mention_count = !prefs.is_muted() && (mentioned || is_dm);
        let should_add_to_thread = is_thread && mentioned;

        Ok(Actions {
            should_push: action.should_push(),
            should_add_to_inbox: action.should_add_to_inbox(),
            should_increment_mention_count,
            should_add_to_thread,
            // notification: notif,
        })
    }

    fn calculate_message_action(
        &self,
        prefs: &Preferences,
        mention_user: bool,
        mention_everyone: bool,
        mention_role: bool,
        reply: bool,
    ) -> Actions {
        if prefs.is_muted() {
            return Actions::skip();
        }

        let reply_action = if reply {
            prefs.resolve_replies().clone().into()
        } else {
            Actions::skip()
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
            NotifsMessages::Everything => Actions::push(),
            NotifsMessages::Watching => {
                if mentioned {
                    Actions::push()
                } else {
                    Actions::inbox()
                }
            }
            NotifsMessages::Mentions => {
                if mentioned {
                    Actions::push()
                } else {
                    Actions::skip()
                }
            }
            NotifsMessages::Nothing => Actions::skip(),
        };

        reply_action.merge(message_action)
    }

    // TODO: fn room_id(&self) -> Option<RoomId>
    // TODO: fn channel_id(&self) -> Option<ChannelId>
}

impl ServiceNotifications {
    pub(super) fn calculator_for_message<'a>(
        &self,
        context: CalculatorContext<'a>,
    ) -> Calculator<'a> {
        Calculator {
            globals: self.globals.clone(),
            context,
        }
    }
}
