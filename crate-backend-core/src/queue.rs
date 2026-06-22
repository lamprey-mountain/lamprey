use async_trait::async_trait;
use common::v2::types::{ChannelId, MessageId, MessageVerId, UserId};
use serde::{Deserialize, Serialize};
use time::Time;
use url::Url;
use uuid::Uuid;

use crate::prelude::*;

// TODO: flesh out these types, use them

// maybe split out retrieving items from queues and inserting items
#[async_trait]
pub trait Queue {
    type Item;

    /// enqueue a new item
    async fn push(&mut self, item: Self::Item) -> Result<()>;

    /// claim an item
    async fn claim(&mut self) -> Result<Self::Item>;

    async fn mark_complete(&mut self, item: &Self::Item) -> Result<()>;
    async fn mark_failed(&mut self, item: &Self::Item) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct QueuedItem<T> {
    pub id: Uuid,
    pub queued_at: Time,
    pub complete_at: Option<Time>,
    pub failed_at: Option<Time>,
    pub retry_count: u8,
    pub data: T,
}

// impl QueuedItem {
//     pub fn map(self<T>, f: T -> U) -> Self<U>
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRef {
    pub message_id: MessageId,
    pub version_id: MessageVerId,
    pub channel_id: ChannelId,
}

#[derive(Debug, Clone)]
pub struct QueuedEmbedGeneration {
    pub message_ref: Option<MessageRef>,
    pub user_id: UserId,
    pub url: Url,
}

#[derive(Debug, Clone)]
pub struct QueuedEmail {
    pub to: String,
    pub from: String,
    pub subject: String,
    pub body_plain: String,
    pub body_html: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueuedNotification {
    // copy DbNotification
}

#[derive(Debug, Clone)]
pub struct QueuedSearch {
    // come up with something new here?
}
