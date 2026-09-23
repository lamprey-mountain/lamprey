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

/// Set of actions to take on an event
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Actions {
    inner: ActionsInner,
}

bitflags::bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    struct ActionsInner: u8 {
        const Push = 1 << 0;
        const Inbox = 1 << 1;
        const IncrementMentions = 1 << 2;
        const IncrementUnreads = 1 << 3;
        const AddToThread = 1 << 4;
    }
}

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
        Self {
            inner: ActionsInner::Inbox,
        }
    }

    /// Send push notification and add to inbox
    pub fn push() -> Self {
        Self {
            inner: ActionsInner::Push | ActionsInner::Inbox,
        }
    }

    /// Whether this notification should be sent as a push notification
    #[inline]
    pub fn should_push(&self) -> bool {
        self.inner.contains(ActionsInner::Push)
    }

    /// Whether this notification should be added to the inbox
    #[inline]
    pub fn should_add_to_inbox(&self) -> bool {
        self.inner.contains(ActionsInner::Inbox)
    }

    /// Whether the mention count should be incremented
    #[inline]
    pub fn should_increment_mention_count(&self) -> bool {
        self.inner.contains(ActionsInner::IncrementMentions)
    }

    /// Whether the unread count should be incremented
    #[inline]
    pub fn should_increment_unread_count(&self) -> bool {
        self.inner.contains(ActionsInner::IncrementUnreads)
    }

    /// Whether the user should be added to the thread
    #[inline]
    pub fn should_add_to_thread(&self) -> bool {
        self.inner.contains(ActionsInner::AddToThread)
    }

    /// Merge these actions with other actions
    pub fn merge(&self, other: Self) -> Self {
        let mut me = Self {
            inner: self.inner | other.inner,
        };

        if me.should_push() {
            me.inner.insert(ActionsInner::Inbox);
        }

        me
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
    ///
    /// Returns the actions that should be done and the notification that should be created for this user
    pub async fn calculate(&self, user_id: UserId) -> Result<(Actions, Notification)> {
        let room_id = self.context.channel.room_id;
        let channel_id = self.context.channel.id;
        let srv = self.globals.services();
        let prefs = srv
            .notifications
            .preferences(user_id, room_id, Some(channel_id))
            .await?;

        let other_user_id = self.context.message.author_id;
        let relationship = self
            .globals
            .begin_read()
            .await?
            .user_relationship_get(user_id, other_user_id)
            .await?
            .unwrap_or_default();
        let ignored = relationship.is_ignored_or_blocked();

        let (actions, notif) = self.calculate_message(user_id, &prefs, ignored);
        Ok((actions, notif))
    }

    fn calculate_message(
        &self,
        user_id: UserId,
        prefs: &Preferences,
        ignored: bool,
    ) -> (Actions, Notification) {
        let mu = &self.context.mentioned_users;
        let mention_user = mu.users_from_direct.contains(&user_id);
        let mention_everyone = mu.users_from_everyone.contains(&user_id);
        let mention_role = mu.users_from_role.contains(&user_id);
        let reply = self
            .context
            .replied_message
            .as_ref()
            .map_or(false, |m| m.author_id == user_id);

        // TODO: handle users_from_recipient?

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

        let message_action = match prefs.resolve_messages() {
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

        let actions = if prefs.is_muted() {
            Actions::skip()
        } else {
            reply_action.merge(message_action)
        };

        let message = &self.context.message;
        let ephemeral = message.ephemeral;
        let channel = &self.context.channel;
        let is_dm = channel.is_dm();
        let is_thread = channel.is_thread();
        let should_increment_mention_count = mentioned || is_dm;
        let should_increment_unread_count = !prefs.is_muted() && is_dm;
        let should_add_to_thread = is_thread && mentioned;

        // PERF: skip calculating actions entirely if ignored
        let actions = if ignored {
            Actions::default()
        } else if ephemeral {
            let mut inner = ActionsInner::empty();
            if actions.should_push() {
                inner.insert(ActionsInner::Push);
            }
            Actions { inner }
        } else {
            let mut inner = ActionsInner::empty();
            if actions.should_push() {
                inner.insert(ActionsInner::Push);
            }
            if actions.should_add_to_inbox() {
                inner.insert(ActionsInner::Inbox);
            }
            if should_increment_mention_count {
                inner.insert(ActionsInner::IncrementMentions);
            }
            if should_increment_unread_count {
                inner.insert(ActionsInner::IncrementUnreads);
            }
            if should_add_to_thread {
                inner.insert(ActionsInner::AddToThread);
            }
            Actions { inner }
        };

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

        (actions, notif)
    }
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
