use std::collections::HashSet;

use common::v1::types::notifications::Notification;
use common::v1::types::{Channel, Message, UserId};
use lamprey_backend_data_postgres::MAX_ROLE_MENTION_MEMBERS;
use tokio::sync::RwLock;

use crate::prelude::*;
use crate::services::notifications::push::VapidKeys;

pub mod ack;
pub mod calculator;
pub mod calculator_old;
pub mod push;

pub struct ServiceNotifications {
    state: Globals,
    vapid_keys: RwLock<Option<VapidKeys>>,
}

#[derive(Debug, Default)]
struct MentionedUsers {
    /// users who were directly mentioned
    users_from_direct: HashSet<UserId>,

    /// users who were mentioned from a role mention
    users_from_role: HashSet<UserId>,

    /// users who were mentioned from an everyone mention
    users_from_everyone: HashSet<UserId>,

    /// users who were mentioned due to being a channel recipient
    users_from_recipient: HashSet<UserId>,
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
    pub async fn process_message(&self, channel: Channel, message: Message) {
        //  ephemeral messages dont create notifications (or insert thread members)
        if message.ephemeral {
            return;
        }

        let mentioned_users = self
            .get_mentioned_users(&channel, &message)
            .await
            .expect("TODO: better error handling");

        // insert thread members
        // PERF: data.thread_member_bulk_insert?
        // emit sync events

        // update ack states (increment mention counts)
        // emit sync events

        // send notifications

        todo!()
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
    pub async fn create(&self, user_id: UserId, notification: Notification) -> Result<()> {
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
