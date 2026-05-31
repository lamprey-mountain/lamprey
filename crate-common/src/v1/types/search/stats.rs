#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{ChannelId, MessageId};

/// Overall search index statistics
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SearchIndexStats {
    /// total number of documents in this index
    pub count_documents: u64,
    pub count_messages: u64,
    pub count_channels: u64,
    pub count_rooms: u64,
    pub count_media: u64,
    pub count_users: u64,
    // TODO: etc...
    /// size of the index in bytes
    pub index_size_bytes: u64,

    /// number of active reindex queues
    pub reindex_queues: u64,
}

/// Search index statistics for a channel
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SearchIndexStatsChannel {
    pub channel_id: ChannelId,

    pub count_documents: u64,
    pub count_messages: u64,
    pub count_media: u64,

    pub last_indexed_message_id: Option<MessageId>,
}

/// Search index statistics for a room
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SearchIndexStatsRoom {
    // TODO
}
