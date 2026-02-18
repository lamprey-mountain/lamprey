use std::collections::HashMap;

use common::v1::types::{ChannelId, Message, MessageCreate, MessageId, MessagePatch, MessageSync};

/// a reference to a single continuous range of messages
pub struct MessageRange<'a> {
    items: &'a [TimelineItem],
    has_forward: bool,
    has_backwards: bool,
}

pub struct MessageRangeFull {
    items: Vec<TimelineItem>,
    has_forward: bool,
    has_backwards: bool,
}

/// a deduplicated set of message ranges for a channel
pub struct MessageRanges {
    /// the id of the channel the messages belong to
    channel_id: ChannelId,

    /// sorted disjoint set of contiguous items
    // the first vec is the live range
    ranges: Vec<MessageRangeFull>,
}

/// a manager for messages known to this client
pub struct Messages {
    ranges: HashMap<ChannelId, MessageRanges>,
}

pub enum TimelineItem {
    /// a message on the server
    Message(Message),

    /// a local message, waiting to be sent
    Local {
        message: Message,

        /// if this failed to send
        failed: bool,
    },
}

impl<'a> MessageRange<'a> {
    /// Returns true if the range contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// returns the `MessageId` of the first item in the range, if this range is not empty
    pub fn start(&self) -> Option<MessageId> {
        self.items.first().map(|item| match item {
            TimelineItem::Message(message) => message.id,
            TimelineItem::Local { message, .. } => message.id,
        })
    }

    /// returns the `MessageId` of the last item in the range, if this range is not empty
    pub fn end(&self) -> Option<MessageId> {
        self.items.last().map(|item| match item {
            TimelineItem::Message(message) => message.id,
            TimelineItem::Local { message, .. } => message.id,
        })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn contains(&self, message_id: MessageId) -> bool {
        self.items.iter().any(|item| match item {
            TimelineItem::Message(message) => message.id == message_id,
            TimelineItem::Local { message, .. } => message.id == message_id,
        })
    }

    pub fn slice(&self, start: usize, end: usize) -> MessageRange {
        MessageRange {
            items: &self.items[start..end],
            has_forward: self.has_forward || end < self.items.len(),
            has_backwards: self.has_backwards || start > 0,
        }
    }

    pub fn items(&self) -> &[TimelineItem] {
        &self.items
    }

    pub fn has_forward(&self) -> bool {
        self.has_forward
    }

    pub fn has_backwards(&self) -> bool {
        self.has_backwards
    }
}

impl MessageRanges {
    /// get the live range that new messages get appended to
    pub fn live(&self) -> MessageRange {
        self.ranges.first().map(|range| MessageRange {
            items: &range.items,
            has_forward: range.has_forward,
            has_backwards: range.has_backwards,
        }).expect("MessageRanges always has at least one range")
    }

    /// find which range a message belongs to
    pub fn find(&self, message_id: MessageId) -> Option<MessageRange> {
        for range in &self.ranges {
            for item in &range.items {
                let id = match item {
                    TimelineItem::Message(message) => message.id,
                    TimelineItem::Local { message, .. } => message.id,
                };
                if id == message_id {
                    return Some(MessageRange {
                        items: &range.items,
                        has_forward: range.has_forward,
                        has_backwards: range.has_backwards,
                    });
                }
            }
        }
        None
    }

    // this is probably internal?
    fn merge(&self, a: MessageRange, b: MessageRange) -> &MessageRange {
        todo!()
    }

    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }
}

impl Messages {
    pub fn fetch_backwards(&self, channel_id: ChannelId, message_id: MessageId, limit: usize) {
        // 1. find range message is in, otherwise create new range
        // 2. if there aren't enough messages, fetch more and merge
        // 3. get a slice of the MessageRange
        todo!()
    }

    // TODO: impl the rest
    pub fn fetch_forwards(&self, channel_id: ChannelId, message_id: MessageId, limit: usize) {
        todo!()
    }

    pub fn fetch_single(&self, channel_id: ChannelId, message_id: MessageId) {
        todo!()
    }

    pub fn fetch_context(&self, channel_id: ChannelId, message_id: MessageId, limit: usize) {
        todo!()
    }

    // TODO: handle local echos
    pub fn send(&self, channel_id: ChannelId, body: MessageCreate) {
        todo!()
    }

    pub fn edit(&self, channel_id: ChannelId, message_id: MessageId, body: MessagePatch) {
        todo!()
    }

    pub(crate) fn handle_sync(&self, sync: MessageSync) {
        todo!()
    }

    // // NOTE: maybe i'll have a subscribe function later, though im not sure if theres any reason to have it
    // pub async fn subscribe(&self) -> impl Stream<Item = ()>;

    // NOTE: i may not need these depending on the impl
    // mergeAfter, mergeBefore
}
