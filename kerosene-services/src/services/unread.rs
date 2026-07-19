// TODO: merge into ServiceNotifications

// TODO: high performance unread handling
//
// Instead of fetching from db, store read states and quickly changing channel data in memory. Flush channel data to postgres occasionally.
// For distributed deployments, I'll probably store read states in nats/jetstream. (how will i handle consistency with flushing to postgres?)

use std::sync::Arc;

use async_nats::jetstream::kv::{Store, UpdateErrorKind};
use bytes::Bytes;
use common::v1::types::{
    Channel, ChannelId, Message, MessageId, MessageSync, MessageType, MessageVerId, NotificationId,
    UserId,
    ack::{Ack, AckBulk, AckCreate, AckState, ChannelAckMetadata},
    notifications::{Notification, NotificationType},
    util::Time,
};
use dashmap::DashMap;
use lamprey_backend_data_postgres::DbChannelPrivate;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::prelude::*;
use crate::{ServerStateInner, services::notifications::calculator::Calculator};

pub struct ServiceUnread {
    state: ServerState2,
    cache_channel: DashMap<ChannelId, ChannelAckMetadata>,
    cache_user_message: DashMap<(ChannelId, UserId), AckStateUserMessage>,
    cache_user_pins: DashMap<(ChannelId, UserId), AckStateUserPins>,
}

#[derive(Debug)]
struct AckStateUserMessage {
    last_read_message_id: MessageId,
    mention_count: u64,
}

#[derive(Debug)]
struct AckStateUserPins {
    last_read_pin_timestamp: Option<Time>,
}

impl ServiceUnread {
    pub fn new(state: ServerState2) -> Self {
        Self {
            state,
            cache_channel: DashMap::new(),
            cache_user_message: DashMap::new(),
            cache_user_pins: DashMap::new(),
        }
    }

    /// acknowledge a channel
    pub async fn ack(&self, user_id: UserId, channel_id: ChannelId, ack: AckCreate) -> Result<()> {
        // update in memory cache
        // update jetstream
        todo!()
    }

    /// acknowledge many channels
    pub async fn ack_bulk(&self, user_id: UserId, acks: AckBulk) -> Result<()> {
        todo!()
    }

    /// handle a sync event
    pub async fn handle_sync(&self, sync: &MessageSync) -> Result<()> {
        match sync {
            MessageSync::MessageCreate { .. } => todo!("handle_message"),
            MessageSync::ChannelCreate { .. } => todo!("handle_channel"),
            // TODO: handle other events
            _ => {}
        }
    }

    /// handle a message
    pub async fn handle_message(&self, message: &Message) -> Result<()> {
        let srv = self.state.services();

        if is_in_dm {
            // if matches!(message type, MessageType::MemberRemove(_)); skip next for loop

            for user in recipients {
                // check that (channel is not muted) or (message mentions user)
                // check that message author is not blocked or ignored by user
                // if true, increment mention_count by 1
            }
        } else {
            let mentioned_users = todo!("get a full list of mentioned users");
            for user in mentioned_users {
                // check that message author is not blocked or ignored by user
                // increment mention_count by 1
            }
        }

        // TODO: reset ack state for message.author_id (mention_count -> 0, last_read_message_id -> chan.last_message_id)

        // TODO: bump channel read metadata (last_message_id)

        Ok(())
    }

    /// handle a channel create event
    pub async fn handle_channel(&self, channel: &Channel) -> Result<()> {
        // if channel is a thread and thread's parent.ty.is_thread_only()
        // then reset ack state for channel.creator_id in channel.parent_id (mention_count -> 0, last_read_message_id -> parent.last_message_id)

        todo!()
    }

    /// save read state in to database
    pub async fn flush(&self) -> Result<()> {
        self.state.messaging().temp_jetstream().await?;
        // PERF: have some way to bulk upsert ack state in database?
        todo!()
    }

    pub fn put_channel(&self, a: ChannelAckMetadata) -> Result<()> {
        todo!()
    }

    pub fn put_user_channel(&self, a: AckStateUserMessage) -> Result<()> {
        todo!()
    }
}

fn a(a: &ChannelAckMetadata) -> Bytes {
    todo!()
}

fn b(a: &AckState) -> Bytes {
    todo!()
}

fn c(bytes: &Bytes) -> Result<ChannelAckMetadata> {
    todo!()
}

fn c(bytes: &Bytes) -> Result<AckState> {
    todo!()
}
