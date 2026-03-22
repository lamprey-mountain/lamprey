// TODO: high performance unread handling
//
// Instead of fetching from db, store read states and quickly changing channel data in memory. Flush channel data to postgres occasionally.
// For distributed deployments, I'll probably store read states in nats/jetstream. (how will i handle consistency with flushing to postgres?)

use std::sync::Arc;

use async_nats::jetstream::kv::{Store, UpdateErrorKind};
use bytes::Bytes;
use common::v1::types::{
    ack::{AckBulk, AckBulkItem},
    Channel, ChannelId, MessageId, MessageVerId, UserId,
};
use dashmap::DashMap;
use lamprey_backend_core::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::ServerStateInner;

pub struct ServiceUnread {
    state: Arc<ServerStateInner>,
    cache_channel: DashMap<ChannelId, ChannelReadMetadata>,
    cache_user: DashMap<(ChannelId, UserId), ReadState>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ChannelReadMetadata {
    pub last_version_id: Option<MessageVerId>,
    pub last_message_id: Option<MessageId>,
    pub message_count: Option<u64>,
    pub root_message_count: Option<u64>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ReadState {
    pub is_unread: bool,
    pub last_read_id: Option<MessageVerId>,
    pub mention_count: u64,
}

impl ChannelReadMetadata {
    pub fn apply(&self, channel: &mut Channel) {
        channel.last_version_id = self.last_version_id;
        channel.last_message_id = self.last_message_id;
        channel.message_count = self.message_count;
        channel.root_message_count = self.root_message_count;
    }
}

impl ReadState {
    pub fn apply(&self, channel: &mut Channel) {
        channel.is_unread = Some(self.is_unread);
        channel.last_read_id = self.last_read_id;
        channel.mention_count = Some(self.mention_count);
    }
}

impl ServiceUnread {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_channel: DashMap::new(),
            cache_user: DashMap::new(),
        }
    }

    pub async fn ack(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        mention_count: u64,
    ) -> Result<()> {
        self.ack_bulk(
            user_id,
            AckBulk {
                acks: vec![AckBulkItem {
                    channel_id,
                    message_id: Some(message_id),
                    version_id,
                    mention_count,
                }],
            },
        )
        .await
    }

    /// acknowledge a bunch of channels at once
    pub async fn ack_bulk(&self, user_id: UserId, acks: AckBulk) -> Result<()> {
        let js = self.state.jetstream.as_ref().unwrap();
        let kv = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "read_states_channels".to_owned(),
                history: 1,
                ..Default::default()
            })
            .await
            .unwrap();
        // let kv = js
        //     .create_key_value(async_nats::jetstream::kv::Config {
        //         bucket: "read_states".to_owned(),
        //         history: 1,
        //         ..Default::default()
        //     })
        //     .await
        //     .unwrap();

        let key = "channel-id-here";
        // let key = "channelid:userid";

        atomic_update(&kv, key, |mut metadata: ChannelReadMetadata| {
            // metadata.last_message_id = Some(new_last_message_id);
            // metadata.message_count = Some(metadata.message_count.unwrap_or(0) + 1);
            metadata
        })
        .await?;

        todo!()
    }
}

async fn atomic_update<T: Default + Serialize + DeserializeOwned, F: Fn(T) -> T>(
    kv: &Store,
    key: &str,
    update: F,
) -> Result<()> {
    loop {
        let (data, revision) = match kv
            .entry(key)
            .await
            .expect("FIXME: add nats error to error enum")
        {
            Some(entry) => {
                let m: T = serde_json::from_slice(&entry.value)?;
                (m, entry.revision)
            }
            None => (T::default(), 0),
        };

        let data = update(data);
        let bytes = Bytes::from(serde_json::to_vec(&data)?);

        match kv.update(key, bytes, revision).await {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == UpdateErrorKind::WrongLastRevision => continue,
            // Err(e) => return Err(e.into()),
            Err(e) => panic!("FIXME: add nats error to error enum"),
        }
    }
}
