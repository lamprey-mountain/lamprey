use std::{num::NonZeroUsize, sync::Arc};

use common::{
    v1::types::{Message, User},
    v2::types::{ChannelId, MessageId, UserId},
};

use crate::cache::Cache;

/// a simple in memory cache
pub struct MemoryCache {
    users: lru::LruCache<UserId, Arc<User>>,
    channels: lru::LruCache<ChannelId, ChannelItem>,
}

struct ChannelItem {
    messages: lru::LruCache<MessageId, Arc<Message>>,
    ranges: lru::LruCache<MessageId, Arc<Message>>,
}

impl MemoryCache {
    /// create a new in memory cache
    pub fn new() -> Self {
        Self {
            users: lru::LruCache::new(NonZeroUsize::new(100).unwrap()),
            channels: todo!(),
        }
    }

    /// use this in memory cache as a layer over another cache
    pub fn layer(self, _other: impl Cache) -> Self {
        todo!()
    }
}

impl Cache for MemoryCache {}
