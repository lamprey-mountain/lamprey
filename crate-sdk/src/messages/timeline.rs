use std::sync::Arc;

use common::{
    v1::types::{Message, MessageCreate},
    v2::types::MessageId,
};

/// an ordered slice of items in a channel
#[derive(Debug, Clone)]
pub struct Timeline {
    items: Vec<Item>,
    has_forward: bool,
    has_backwards: bool,
    stale: bool,
}

#[derive(Debug, Clone)]
pub enum Item {
    Message(Arc<Message>),
    Pending(Arc<Pending>),
    // TODO: add ui things?
    // Divider, // unread + timestamp
    // Header,
}

#[derive(Debug, Clone)]
pub struct Pending {
    nonce: String,
    create: MessageCreate,
    status: PendingStatus,
}

#[derive(Debug, Clone)]
pub enum PendingStatus {
    Sending,
    Failed,
    // TODO: more descriptive failure/error messages Failed(Arc<Error>),
}

impl Timeline {
    /// returns true if this slice contains no messages
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// returns the id of the first message in the slice, if any
    #[inline]
    pub fn start(&self) -> Option<MessageId> {
        self.items.iter().find_map(Item::id)
    }

    /// returns the id of the last message in the slice, if any
    #[inline]
    pub fn end(&self) -> Option<MessageId> {
        self.items.iter().rev().find_map(Item::id)
    }

    /// returns true if the given message id is within the range of this slice
    pub fn contains(&self, id: MessageId) -> bool {
        let (Some(first), Some(last)) = (self.start(), self.end()) else {
            return false;
        };

        first <= id && id <= last
    }

    /// the number of messages in this slice
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// returns a new `MessageSlice` containing a subset of the messages in this slice
    // TODO: allow using slice syntax? (core::ops::something)
    pub fn slice(&self, start: usize, end: usize) -> Timeline {
        Timeline {
            items: self.items[start..end].to_vec(),
            has_forward: self.has_forward,
            has_backwards: self.has_backwards,
            stale: self.stale,
        }
    }

    /// whether there are more (possibly unloaded) messages before the start of this slice
    #[inline]
    pub fn has_backwards(&self) -> bool {
        self.has_backwards
    }

    /// whether there are more (possibly unloaded) messages after the end of this slice
    #[inline]
    pub fn has_forwards(&self) -> bool {
        self.has_forward
    }
}

impl Item {
    /// get the message id, if this is a message
    pub fn id(&self) -> Option<MessageId> {
        match self {
            Item::Message(m) => Some(m.id),
            Item::Pending { .. } => None,
        }
    }
}
