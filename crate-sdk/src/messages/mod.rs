#![allow(unused)] // TEMP: while prototyping

use core::fmt;
use std::collections::BTreeMap;
use std::sync::Arc;

use common::v1::types::{ChannelId, Message, MessageCreate, MessageId, MessagePatch};
use parking_lot::RwLock;
use tokio::sync::watch;

use crate::Client;
use crate::cache::CachedChannel;
use crate::http::Http;
use crate::prelude::*;

mod range;
mod timeline;

pub use range::Range;
pub use timeline::{Item as TimelineItem, Timeline};

/// Manager for messages in a channel
#[derive(Clone)]
pub struct Messages {
    http: Http,
    chan: Arc<CachedChannel>,
}

/// a handle to a `MessagesCache`
#[derive(Debug)]
pub struct MessagesHandle {
    state: RwLock<MessagesCache>,

    /// bumped on every change
    generation: watch::Sender<u64>,
}

/// a channel's cached message state
// TODO: store message versions
// TODO: use a lru/lfu cache to expire unused messages? or at least have some way to reclaim memory.
#[derive(Debug, Default)]
pub(crate) struct MessagesCache {
    /// every known message in this channel
    messages: BTreeMap<MessageId, Arc<Message>>,

    /// sorted and disjoint list of loaded message ranges
    ranges: Vec<Range>,

    // /// pending messages without server ids
    // local: Vec<Local>,
    /// the very first message id in this channel, if it is known
    start: Option<MessageId>,

    /// the very last message id in this channel, if it is known
    end: Option<MessageId>,
    // messages: lru::LruCache<MessageId, Arc<Message>>,
    // ranges: lru::LruCache<MessageId, Arc<Message>>,

    // generation: u64,
    // tombstones: VecDeque<MessageId>, unsure if this is necessary
}

impl MessagesHandle {
    pub(crate) fn new() -> Self {
        let (generation, _rx) = watch::channel(0u64);
        Self {
            state: RwLock::new(MessagesCache::default()),
            generation,
        }
    }
}

impl Messages {
    /// fetch a single message
    pub async fn fetch(&self, _id: MessageId) -> Result<Arc<Message>> {
        todo!()
    }

    /// fetch a range of messages before this message
    ///
    /// the returned `MessageRange` may contain more or less messages than `limit`
    pub async fn fetch_before(&self, _id: MessageId, _limitt: u16) -> Result<Timeline> {
        todo!()
    }

    pub async fn fetch_after(&self, _id: MessageId, _limitt: u16) -> Result<Timeline> {
        todo!()
    }

    pub async fn fetch_context(&self, _id: MessageId, _limit: u16) -> Result<Timeline> {
        todo!()
    }

    pub async fn send(&self, _create: MessageCreate) -> Result<Arc<Message>> {
        todo!()
    }

    // TODO: allow editing in flight (local) messages
    pub async fn edit(&self, _id: MessageId, _patch: MessagePatch) -> Result<Arc<Message>> {
        todo!()
    }

    // /// subscribe to the live timeline
    // ///
    // /// returns a stream of message slices containing the last `limit` messages in this channel
    // pub fn subscribe_live(&self, limit: u16) -> impl Stream<Item = Timeline> {
    //     // TODO: subscribe_live -> Subscription
    //     // impl Subscription {
    //     // fn messages() -> stream of messages
    //     // fn items() -> stream of items
    //     // fn timelines() -> stream of timelines
    //     // }
    //     futures_util::stream::empty().boxed()
    // }
}

impl fmt::Debug for Messages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: better debugging (eg. message count)
        f.debug_struct("Messages")
            .field("channel_id", &self.chan.inner.load().id)
            .finish()
    }
}

impl Client {
    /// Get a handle to the messages for a channel.
    pub fn messages(&self, channel_id: ChannelId) -> Messages {
        let cache = self.cache().expect("cache is required");
        Messages {
            http: self.http(),
            // TODO: return None/Err instead of panicing
            chan: cache.channel(channel_id).unwrap(),
        }
    }
}
