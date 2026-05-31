use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use common::v1::types::util::Time;
use common::v1::types::{ChannelId, MessageCreate, MessageId, UserId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPurgeCache {
    pub targets: Vec<AdminPurgeCacheTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPurgeCacheResponse {
    pub stats: Vec<AdminPurgeCacheStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPurgeCacheStat {
    pub target: AdminPurgeCacheTarget,
    pub bytes_reclaimed: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum AdminPurgeCacheTarget {
    Channels,
    Embeds,
    Permissions,
    Rooms,
    Sessions,
    Users,
    // NOTE: add more targets here as caches are added!
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminCollectGarbage {
    pub targets: Vec<AdminCollectGarbageTarget>,
    pub mode: AdminCollectGarbageMode,

    /// whether to return 202 accepted or calculate stats
    #[serde(rename = "async")]
    pub async_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminCollectGarbageResponse {
    pub stats: Vec<AdminCollectGarbageStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminCollectGarbageStat {
    pub target: AdminCollectGarbageTarget,

    /// The number of milliseconds taken to run this garbage collection task.
    pub ms_elapsed: u64,

    /// The number of rows that were deleted (or would be deleted)
    pub rows_deleted: u64,

    /// Number of bytes deleted (or would be deleted); only returned for the `Media` target.
    // TODO: skip serializing if none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_deleted: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum AdminCollectGarbageTarget {
    Media,
    Messages,
    Session,
    AuditLog,
    RoomAnalytics,
    // NOTE: add more targets here as more garbage collectable resources are added!
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum AdminCollectGarbageMode {
    /// Dry-run mode. Calculate stats, but don't touch the database at all.
    Dry,

    /// Set `deleted_at` for all records that should be garbage collected
    Mark,

    /// Delete all records with `deleted_at` set. Note that `Mark` will need to be run first to do anything.
    Sweep,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminWhisper {
    pub user_id: UserId,
    pub message: MessageCreate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminBroadcast {
    pub message: MessageCreate,
    // TODO: add these
    // /// only broadcast to users in these rooms
    // room_id: Vec<RoomId>,

    // /// only broadcast to these users
    // user_id: Vec<UserId>,

    // /// only broadcast to these users with these server roles
    // server_roles: Vec<RoleId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminRegisterUser {
    pub user_id: UserId,
}

/// Overall search index statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchIndexStats {
    /// total number of documents in this index
    pub count_documents: u64,
    pub count_messages: u64,
    pub count_channels: u64,
    pub count_rooms: u64,
    pub count_media: u64,
    pub count_users: u64,
    // etc...
    /// Size of the index in bytes
    pub index_size_bytes: u64,

    /// number of active reindex queues
    pub reindex_queues: u64,
}

/// Search index statistics for a channel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchIndexStatsChannel {
    pub channel_id: ChannelId,

    pub count_documents: u64,
    pub count_messages: u64,
    pub count_media: u64,

    pub last_indexed_message_id: Option<MessageId>,
}

pub struct SearchIndexStatsRoom {
    // TODO
}

/// A dead letter queue entry for search ingestion
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DlqEntry {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub error_message: String,
    pub created_at: Time,
}
