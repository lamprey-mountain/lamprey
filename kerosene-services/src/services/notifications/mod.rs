use std::collections::HashSet;

use common::v1::types::notifications::Notification;
use common::v1::types::{Channel, Message, MessageSync, UserId};
use lamprey_backend_data_postgres::MAX_ROLE_MENTION_MEMBERS;
use tokio::sync::RwLock;
use tracing::warn;

use crate::prelude::*;
use crate::services::notifications::push::VapidKeys;

pub mod ack;
pub mod calculator;
pub mod push;

pub struct ServiceNotifications {
    state: Globals,
    vapid_keys: RwLock<Option<VapidKeys>>,
}

#[derive(Debug, Default)]
pub struct MentionedUsers {
    /// users who were directly mentioned
    pub users_from_direct: HashSet<UserId>,

    /// users who were mentioned from a role mention
    pub users_from_role: HashSet<UserId>,

    /// users who were mentioned from an everyone mention
    pub users_from_everyone: HashSet<UserId>,

    /// users who were mentioned due to being a channel recipient
    pub users_from_recipient: HashSet<UserId>,
}

impl MentionedUsers {
    pub fn all(&self) -> HashSet<UserId> {
        let mut all = HashSet::new();
        all.extend(&self.users_from_direct);
        all.extend(&self.users_from_role);
        all.extend(&self.users_from_everyone);
        all.extend(&self.users_from_recipient);
        all
    }
}

impl ServiceNotifications {
    pub fn new(state: Globals) -> Self {
        Self {
            state,
            vapid_keys: RwLock::new(None),
        }
    }

    pub fn start_background_tasks(&self) {
        tokio::spawn(Self::spawn_push_task(self.state.clone()));
    }

    // TODO: flush ack states on shutdown

    // NOTE: should ServiceNotifications *really* be in charge of inserting thread members?
    // TODO: fn process_message_inner -> Result, make process_message do logging
    pub async fn process_message(&self, channel: Channel, message: Message) {
        //  ephemeral messages dont create notifications (or insert thread members)
        // TODO: move this logic into calculator
        if message.ephemeral {
            return;
        }

        let calc =
            match calculator::Calculator::load_for_message(self.state.clone(), &channel, &message)
                .await
            {
                Ok(c) => c,
                Err(err) => {
                    warn!("failed to load calculator: {err:?}");
                    return;
                }
            };

        // PERF: don't get_mentioned_users twice (Calculator::load_for_message also calls this)
        let mentioned_users = self
            .get_mentioned_users(&channel, &message)
            .await
            .unwrap_or_default(); // TODO: better error logging

        let targets: Vec<UserId> = mentioned_users
            .all()
            .into_iter()
            .filter(|&id| id != message.author_id) // TODO: move this logic into calculator
            .collect();
        if targets.is_empty() {
            return;
        }

        let mut data = match self.state.begin().await {
            Ok(d) => d,
            Err(err) => {
                warn!("failed to begin database transaction, skipping: {err:?}");
                return;
            }
        };

        let mut users_to_increment = vec![];
        for target in &targets {
            let action = match calc.calculate(*target).await {
                Ok(a) => a,
                Err(err) => {
                    warn!("failed to calculate actions for user {target:?}: {err:?}");
                    continue;
                }
            };

            if action.should_increment_mention_count() {
                users_to_increment.push(*target);
            }

            if action.should_add_to_inbox() {
                if let Some(notification) = action.notification() {
                    if let Err(err) = data.notification_add(*target, notification.clone()).await {
                        warn!("failed to add notification: {err:?}");
                    }
                }
            }

            // FIXME: handle action.should_push()
            // TODO: add action.should_add_to_thread()
        }

        let mut thread_members = vec![];
        if channel.ty.is_thread() {
            let _ = data.thread_member_put_bulk(channel.id, &targets).await;
            thread_members = data
                .thread_member_get_many(channel.id, &targets)
                .await
                .unwrap_or_default();
        }

        if !users_to_increment.is_empty() {
            if let Err(err) = data
                .unread_increment_counts(channel.id, &users_to_increment, &[])
                .await
            {
                warn!("failed to increment unread counts: {err:?}");
            }
        }

        if let Err(err) = data.commit().await {
            warn!("failed to commit database transaction: {err:?}");
        }

        let srv = self.state.services();
        if !thread_members.is_empty() {
            let thread_id = channel.id;

            srv.channels.invalidate(thread_id).await;

            let msg = MessageSync::ThreadMemberUpsert {
                room_id: channel.room_id,
                thread_id,
                added: thread_members,
                removed: vec![],
            };

            if let Err(err) = self
                .state
                .messaging()
                .broadcast_channel(thread_id, msg)
                .await
            {
                warn!("failed to broadcast thread member update: {err:?}");
            }
        }
    }

    /// get a set of ids of all users who were mentioned in a message
    async fn get_mentioned_users(
        &self,
        channel: &Channel,
        message: &Message,
    ) -> Result<MentionedUsers> {
        let mut m = MentionedUsers::default();
        let mentions = &message.latest_version.mentions;
        let mut data = self.state.begin_read().await?;

        // add user mentions
        for u in &mentions.users {
            m.users_from_direct.insert(u.id);
        }

        // add recipients for dms
        if channel.ty.is_dm() {
            for recipient in &channel.recipients {
                m.users_from_recipient.insert(recipient.id);
            }
        }

        if mentions.everyone || !mentions.roles.is_empty() {
            let is_thread = channel.ty.is_thread();

            // collect role mentions
            for r in &mentions.roles {
                // TODO: read members from room actor, filter by role
                if let Ok(members) = data.role_member_list(r.id, Default::default()).await {
                    if !is_thread || members.items.len() as u32 <= MAX_ROLE_MENTION_MEMBERS {
                        for member in members.items {
                            m.users_from_role.insert(member.user_id);
                        }
                    }
                }
            }

            // collect everyone mentions
            if mentions.everyone {
                let everyone_ids = if is_thread {
                    data.thread_member_list_all(channel.id)
                        .await
                        .ok()
                        .map(|members| members.into_iter().map(|u| u.user_id).collect::<Vec<_>>())
                } else if let Some(room_id) = channel.room_id {
                    data.room_member_list_all(room_id)
                        .await
                        .ok()
                        .map(|members| members.into_iter().map(|u| u.user_id).collect::<Vec<_>>())
                } else {
                    // TODO: handle dms (@everyone mentions all recipients in dms/gdms)
                    None
                };

                if let Some(ids) = everyone_ids {
                    m.users_from_everyone.extend(ids);
                }
            }
        }

        Ok(m)
    }

    /// add a notification
    ///
    /// inserts into the database and executes any needed actions
    pub async fn create(&self, _user_id: UserId, _notification: Notification) -> Result<()> {
        todo!()
        // let action = self.calculate_actions(user_id, &notification).await?;

        // match (action.should_add_to_inbox(), action.should_push()) {
        //     (true, true) => {
        //         let mut data = self.state.acquire_data().await?;
        //         data.notification_add(user_id, notification).await?;
        //         data.commit().await?;
        //     }
        //     (true, false) => {
        //         let mut data = self.state.acquire_data().await?;
        //         data.notification_add(user_id, notification.clone()).await?;
        //         data.notification_set_pushed(&[notification.id]).await?;
        //         data.commit().await?;
        //     }
        //     (false, true) => {
        //         // NOTE: this branch currently isn't reachable
        //         // TODO: durable pushing without adding to inbox
        //         todo!()
        //     }
        //     (false, false) => {}
        // }

        // Ok(())
    }
}
