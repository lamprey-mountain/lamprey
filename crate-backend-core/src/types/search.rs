use common::{
    v1::types::{ChannelId, search::Doctype},
    v2::types::RoomId,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// a request to reindex stuff on the server
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Reindex {
    /// reindex only these types of documents
    pub doctypes: Vec<Doctype>,

    /// reindex only documents in these rooms
    ///
    /// includes the room iself. empty vec means no filter.
    pub room_ids: Vec<RoomId>,

    /// reindex only documents in these channels
    ///
    /// includes the channel iself. empty vec means no filter.
    pub channel_ids: Vec<ChannelId>,
}

/// a queue that needs to be reindexed
#[derive(Debug, Clone)]
pub struct SearchReindexQueue {
    pub target: SearchReindexQueueTarget,
    pub last_item_id: Option<Uuid>,
}

/// the target of a search reindex queue
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SearchReindexQueueTarget {
    /// messages in a channel
    Messages(ChannelId),

    /// channels on the server
    Channels,

    /// rooms on the server
    Rooms,

    /// users on the server
    Users,

    /// media on the server
    Media,

    /// audit log entries in a room
    AuditLogEntries(RoomId),
}

impl Reindex {
    /// check if this reindex request is empty (no filters)
    pub fn is_empty(&self) -> bool {
        self.doctypes.is_empty() && self.room_ids.is_empty() && self.channel_ids.is_empty()
    }

    /// create a new [`Reindex`] operation to reindex everything
    pub fn everything() -> Self {
        Self {
            doctypes: vec![],
            room_ids: vec![],
            channel_ids: vec![],
        }
    }
}
