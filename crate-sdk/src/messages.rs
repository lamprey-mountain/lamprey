// TODO: use Arc::clone instead of .clone()

use std::collections::BTreeMap;
use std::sync::Arc;

use common::v1::types::{
    ChannelId, ContextQuery, Message, MessageCreate, MessageId, MessagePatch, MessagePayload,
    MessageSync, PaginationDirection, PaginationQuery,
};
use futures_util::{Stream, StreamExt};
use tokio::sync::RwLock;

use crate::Client;
use crate::http::Http;
use crate::prelude::*;
use crate::syncer::{SyncerEvent, SyncerHandle};

/// the messages in a channel
#[derive(Debug, Clone)]
pub struct Messages {
    channel_id: ChannelId,
    http: Http,
    inner: Arc<RwLock<MessagesInner>>,
}

#[derive(Debug)]
struct MessagesInner {
    // TODO: message versions
    /// every known message in this channel
    messages: BTreeMap<MessageId, Arc<Message>>,

    /// sorted and disjoint list of loaded message ranges
    ranges: Vec<Range>,

    /// pending messages without server ids
    local: Vec<Local>,

    /// the very first message id in this channel, if it is known
    start: Option<MessageId>,
}

/// a range of messages that are known to be loaded
#[derive(Debug, Clone, Copy)]
struct Range {
    /// the start of this interval (inclusive)
    start: MessageId,

    /// the end of this interval (inclusive)
    end: MessageId,

    /// whether this range is stale
    ///
    /// stale ranges can be used in the ui while loading, but MUST be replaced
    /// with fresh data from the server if it is received. eg. if paginating
    /// backwards, don't reuse stale ranges; instead fetch and write over them.
    stale: bool,
}

/// a local message in the process of being sent
#[derive(Debug)]
struct Local {
    create: MessageCreate,
    nonce: String,
}

#[derive(Debug)]
pub struct MessageSlice {
    messages: Vec<Arc<Message>>,
    // has_forward: bool,
    // has_backwards: bool,
    // stale: bool,
}

// NOTE: maybe allow multiple in flight requests as long as they're deduplicated?
impl Messages {
    /// fetch a single message
    pub async fn fetch(&self, id: MessageId) -> Result<Arc<Message>> {
        {
            let read = self.inner.read().await;
            if let Some(m) = read.messages.get(&id) {
                return Ok(m.clone());
            }
        }

        let message = Arc::new(
            self.http
                .message_get(self.channel_id, id)
                .await
                .map_err(|e| anyhow::anyhow!("failed to fetch message: {}", e))?,
        );

        {
            let mut write = self.inner.write().await;
            write.insert_message(message.clone());
            write.insert_range(Range {
                start: id,
                end: id,
                stale: false,
            });
        }

        Ok(message)
    }

    /// fetch a range of messages before this message
    ///
    /// the returned `MessageRange` may contain more or less messages than `limit`
    pub async fn fetch_before(&self, id: MessageId, limit: u16) -> Result<MessageSlice> {
        {
            let read = self.inner.read().await;
            if let Some(range) = read.find_range(id) {
                let have: Vec<_> = read
                    .messages
                    .range(range.start..=id)
                    .take(limit as usize)
                    .map(|(_, m)| m.clone())
                    .collect();
                if have.len() == limit as usize || Some(range.start) == read.start {
                    return Ok(MessageSlice { messages: have });
                }
            }
        }

        let res = self
            .http
            .message_list(
                self.channel_id,
                &PaginationQuery {
                    from: Some(id),
                    to: None,
                    dir: Some(PaginationDirection::B),
                    limit: Some(limit),
                },
            )
            .await?;
        let msgs: Vec<Arc<Message>> = res.items.into_iter().map(Arc::new).collect();
        let new_range = Range {
            start: msgs.first().map(|m| m.id).unwrap_or(id),
            end: id,
            stale: false,
        };

        {
            let mut write = self.inner.write().await;
            for m in msgs.iter().cloned() {
                write.insert_message(m);
            }
            write.insert_range(new_range);
        }

        Ok(MessageSlice { messages: msgs })
    }

    pub async fn fetch_after(&self, id: MessageId, limit: u16) -> Result<MessageSlice> {
        {
            let read = self.inner.read().await;
            if let Some(range) = read.find_range(id) {
                // messages in range after id
                let have: Vec<_> = read
                    .messages
                    .range(id..=range.end)
                    .take(limit as usize)
                    .map(|(_, m)| m.clone())
                    .collect();
                if have.len() == limit as usize {
                    return Ok(MessageSlice { messages: have });
                }
            }
        }

        let res = self
            .http
            .message_list(
                self.channel_id,
                &PaginationQuery {
                    from: Some(id),
                    to: None,
                    dir: Some(PaginationDirection::F),
                    limit: Some(limit),
                },
            )
            .await?;
        let msgs: Vec<Arc<Message>> = res.items.into_iter().map(Arc::new).collect();
        let new_range = Range {
            start: id,
            end: msgs.last().map(|m| m.id).unwrap_or(id),
            stale: false,
        };

        {
            let mut write = self.inner.write().await;
            for m in msgs.iter().cloned() {
                write.insert_message(m);
            }
            write.insert_range(new_range);
        }

        Ok(MessageSlice { messages: msgs })
    }

    pub async fn fetch_context(&self, id: MessageId, limit: u16) -> Result<MessageSlice> {
        // TODO: return existing messge slice if it exists

        let res = self
            .http
            .message_context(
                self.channel_id,
                id,
                ContextQuery {
                    to_start: None,
                    to_end: None,
                    limit: Some(limit),
                },
            )
            .await?;

        let msgs: Vec<Arc<Message>> = res.items.into_iter().map(Arc::new).collect();

        if !msgs.is_empty() {
            let new_range = Range {
                start: msgs.first().unwrap().id,
                end: msgs.last().unwrap().id,
                stale: false,
            };

            let mut write = self.inner.write().await;
            for m in msgs.iter().cloned() {
                write.insert_message(m);
            }
            write.insert_range(new_range);
        }

        Ok(MessageSlice { messages: msgs })
    }

    pub async fn send(&self, create: MessageCreate) -> Result<&Message> {
        // 1. immediately insert into self.segments and update subscribers

        // 2. send message
        let message = self.http.message_create(self.channel_id, &create).await?;

        // 3. update self.segments, merge ranges if needed
        // NOTE: whether the http route returns first or the websocket sends a message first is a race condition, i need to handle both

        // 4. return sent message

        todo!()
    }

    // TODO: allow editing in flight (local) messages
    pub async fn edit(&self, id: MessageId, patch: MessagePatch) -> Result<&Message> {
        todo!()
    }

    /// subscribe to the live timeline
    ///
    /// returns a stream of message slices containing the last `limit` messages in this channel
    pub fn subscribe_live(&self, limit: u16) -> impl Stream<Item = MessageSlice> {
        futures_util::stream::empty().boxed()
    }

    // // ???
    // // maybe have subscribe_foo variants of fetch_foo?
    // pub fn subscribe_range(&self, limit: u16) -> impl Stream<Item = MessageSlice<'_>> {
    //     futures_util::stream::empty().boxed()
    // }
}

impl MessagesInner {
    fn insert_message(&mut self, message: Arc<Message>) {
        self.messages.insert(message.id, message);
    }

    /// insert a new range, merging with overlapping/adjacent ranges
    fn insert_range(&mut self, new_range: Range) {
        let idx = self.ranges.partition_point(|r| r.end < new_range.start);

        let mut merged = new_range;
        let mut remove_start = idx;
        let mut remove_end = idx;

        // merge backwards
        while remove_start > 0 && self.ranges[remove_start - 1].touches(merged) {
            merged = merged.merge(self.ranges[remove_start - 1]);
            remove_start -= 1;
        }

        // merge forwards
        while remove_end < self.ranges.len() && self.ranges[remove_end].touches(merged) {
            merged = merged.merge(self.ranges[remove_end]);
            remove_end += 1;
        }

        self.ranges.splice(remove_start..remove_end, [merged]);
    }

    /// get which range this message id is in
    fn find_range(&self, id: MessageId) -> Option<Range> {
        let idx = self.ranges.partition_point(|r| r.end < id);
        self.ranges.get(idx).filter(|r| r.contains(id)).copied()
    }
}

impl Range {
    /// construct a new [`Range`] containing a single (non stale) message id
    pub fn single(id: MessageId) -> Self {
        Self {
            start: id,
            end: id,
            stale: false,
        }
    }

    /// whether this range and another range are overlapping or adjacent (should merge)
    pub fn touches(&self, other: Range) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    pub fn contains(&self, id: MessageId) -> bool {
        self.start <= id && id <= self.end
    }

    /// merge this range with another range
    pub fn merge(&self, other: Range) -> Range {
        Range {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            stale: self.stale && other.stale,
        }
    }
}

impl MessageSlice {
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    pub fn start(&self) -> Option<MessageId> {
        todo!()
    }

    pub fn end(&self) -> Option<MessageId> {
        todo!()
    }

    pub fn contains(&self, id: MessageId) -> bool {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    // TODO: allow using slice syntax? (core::ops::something)
    pub fn slice(&self, start: usize, end: usize) -> MessageSlice {
        todo!()
    }

    /// whether there are more (possibly unloaded) messages before the start of this slice
    pub fn has_backwads(&self) -> bool {
        todo!()
    }

    /// whether there are more (possibly unloaded) messages after the end of this slice
    pub fn has_forwards(&self) -> bool {
        todo!()
    }
}

async fn spawn_sync_task(syncer: SyncerHandle, inner: Arc<RwLock<MessagesInner>>) {
    let mut s = syncer.subscribe();
    while let Some(e) = s.next().await {
        match &*e {
            SyncerEvent::Message(m) => match &m.payload {
                MessagePayload::Sync { data, nonce, .. } => match &**data {
                    MessageSync::MessageCreate { message } => {
                        // NOTE: if nonce == idempotency_key we sent this message
                        // let a = inner.write().await;
                        // a.insert_message(message);
                        // a.insert_range(new_range);
                        // Range::single(message.id)
                        todo!()
                    }
                    MessageSync::MessageUpdate { message } => todo!(),
                    MessageSync::MessageDelete {
                        channel_id,
                        message_id,
                    } => todo!(),
                    MessageSync::MessageDeleteBulk {
                        channel_id,
                        message_ids,
                    } => todo!(),
                    MessageSync::MessageVersionDelete {
                        channel_id,
                        message_id,
                        version_id,
                    } => todo!(),
                    MessageSync::MessageRemove {
                        channel_id,
                        message_ids,
                    } => todo!(),
                    MessageSync::MessageRestore {
                        channel_id,
                        message_ids,
                    } => todo!(),
                    _ => {}
                },
                _ => {}
            },
            SyncerEvent::StateChanged => {
                todo!("mark as stale if disconnected and failed to resume")
            }
            _ => {}
        }
    }
}

impl Client {
    pub async fn messages(&self, channel_id: ChannelId) -> Messages {
        Messages {
            channel_id,
            http: self.http(),
            inner: todo!(),
        }
    }
}
