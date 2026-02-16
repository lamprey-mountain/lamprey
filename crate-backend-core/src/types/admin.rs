use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
