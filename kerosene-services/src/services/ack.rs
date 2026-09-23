use common::v1::types::MessageSync;
use common::v1::types::ack::{AckBulk, AckBulkItem, AckCreate, AckState, AckType};
use common::v1::types::util::Time;
use common::v2::types::{ChannelId, MessageId, MessageVerId, UserId};
use dashmap::DashMap;
use moka::future::Cache;

use crate::prelude::*;
use crate::services::notifications::ServiceNotifications;

/// a user's full read state
#[derive(Debug, Default)]
pub struct UserAcks {
    /// per channel read state
    channels: DashMap<ChannelId, ChannelAcks>,
}

/// a user's read state for a channel
#[derive(Debug, Default)]
pub struct ChannelAcks {
    // PERF: ensure MessageId is non-zero (niche)
    last_read_id: Option<MessageId>,
    mention_count: u64,
    unread: bool,
    pins_read_at: Option<Time>,
}

/// channel unread state
#[derive(Debug, Default)]
pub struct ChannelState {
    last_message_id: Option<MessageId>,
    // TODO: deprecate and remove
    last_version_id: Option<MessageVerId>,
    pins_updated_at: Option<Time>,
}

pub struct ServiceAck {
    globals: Globals,
    channels: Cache<ChannelId, Arc<ChannelState>>,
    acks: Cache<UserId, Arc<ChannelState>>,
}

// PERF(future): store unreads in nats jetstream
// im not sure if im going to continue using nats or not though

impl ServiceAck {
    pub fn new(globals: Globals) -> Self {
        // NOTE: do i need support_invalidation_closures()?
        Self {
            globals,
            channels: Cache::builder().max_capacity(100_000).build(),
            acks: Cache::builder().max_capacity(100_000).build(),
        }
    }

    /// write all changes to the database
    pub async fn flush(&self) -> Result<()> {
        // TODO: automatically flush on a timer or after a certain number of unflushed updates
        // maybe store updates as an operation log? maybe a list of `MessageSync`s?

        // TODO: flush ack states on shutdown

        // enum Operation {
        //     MessageCreate(Message),
        //     MessagePinned(Message),
        //     ThreadCreate(Channel),
        // }

        todo!()
    }

    /// update a user's ack state for a channel
    pub async fn ack(&self, user_id: UserId, channel_id: ChannelId, ack: AckCreate) -> Result<()> {
        let item = AckBulkItem {
            ty: AckType::Message {
                channel_id,
                message_id: ack
                    .message_id
                    .unwrap_or_else(|| todo!("load channel ack state, get last_message_id")),
                mention_count: ack.mention_count,
            },
        };
        self.bulk(user_id, &AckBulk { acks: vec![item] }).await
    }

    /// update a user's ack state en masse
    pub async fn bulk(&self, user_id: UserId, acks: &AckBulk) -> Result<()> {
        // let mut channel_ids = HashSet::new();
        // let mut unknown_auth = false;

        // for ack in &acks {
        //     if let Some(channel_id) = ack.ty.channel_id() {
        //         channel_ids.insert(channel_id);
        //     } else {
        //         unknown_auth = true;
        //     }
        // }

        // if unknown_auth {
        //     warn!("unknown auth check for this ack type, allowing");
        // }

        // for &channel_id in &channel_ids {
        //     srv.perms
        //         .for_channel3(Some(auth.user.id), channel_id)
        //         .await?
        //         .ensure_view()?
        //         .check()?;
        // }

        // if !req.body.acks.is_empty() {
        //     data.unread_ack_bulk(auth.user.id, &req.body.acks).await?;

        //     for &channel_id in &channel_ids {
        //         srv.channels.invalidate_user(channel_id, auth.user.id).await;
        //     }

        //     s.broadcast(MessageSync::PassiveAck {
        //         user_id: auth.user.id,
        //         ack_states: req
        //             .body
        //             .acks
        //             .into_iter()
        //             .map(|a| AckState {
        //                 ty: a.ty,
        //                 unread: false,
        //             })
        //             .collect(),
        //     })?;
        // }

        // MessageSync::PassiveAck { user_id, ack_states: vec![] };
        // MessageSync::PassiveRoom { user_id, room_id: (), ack_states: vec![AckState { ty: todo!(), unread: todo!() }], voice_states: () };

        todo!()
    }

    /// update ack caches via a sync event
    pub async fn handle(&self, sync: &MessageSync) -> Result<()> {
        match sync {
            MessageSync::MessageCreate { message } => {
                // if is_in_dm {
                //     if matches!(message type, MessageType::MemberRemove(_)); skip next for loop
                //
                //     for user in recipients {
                //         // check that (channel is not muted) or (message mentions user)
                //         // check that message author is not blocked or ignored by user
                //         // if true, increment mention_count by 1
                //     }
                // } else {
                //     let mentioned_users = todo!("get a full list of mentioned users");
                //     for user in mentioned_users {
                //         // check that message author is not blocked or ignored by user
                //         // increment mention_count by 1
                //     }
                // }

                // TODO: reset ack state for message.author_id (mention_count -> 0, last_read_message_id -> chan.last_message_id)

                // TODO: bump channel read metadata (last_message_id)
            }
            MessageSync::MessageUpdate { message } => {
                // handle pins updates
            }
            MessageSync::ChannelCreate { channel } => {
                if channel.ty.is_thread() {
                    // if parent.ty.is_thread_only()
                    // 1. reset ack state for channel.creator_id in channel.parent_id (mention_count -> 0, last_read_message_id -> parent.last_message_id)
                    // 2. update last_message_id (?)
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// increment the mention count for all of these users in a channel
    pub(crate) async fn increment_mentions(
        &self,
        channel_id: ChannelId,
        user_ids: &[UserId],
    ) -> Result<()> {
        let mut txn = self.globals.begin().await?;
        txn.unread_increment_counts(channel_id, &user_ids, &[])
            .await?;
        txn.commit().await?;
        Ok(())
    }
}
