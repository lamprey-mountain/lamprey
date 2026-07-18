use lamprey_macros::record;

use crate::v1::types::{SfuId, UserId};

// TODO: use this
#[record]
pub struct Sfu {
    pub id: SfuId,

    /// human readable name (for debugging)
    pub name: String,

    /// if this sfu is selfhosted
    pub external: Option<SfuExternal>,

    /// the latest statistics for this sfu
    pub stats: SfuStats,
    // TODO: maybe add these?
    // is_available: bool,
    // is_optimal: bool,
}

// TODO: use this
#[record]
pub struct SfuExternal {
    /// the id of the user who is running this sfu
    pub user_id: UserId,

    pub motd: Option<String>,
}

/// statistics for a sfu
#[record]
pub struct SfuStats {
    /// the number of peers connected to this sfu
    pub peer_count: u64,

    /// currently used bandwidth in bits per second
    pub bandwidth_usage: u64,

    /// maximum available bandwidth in bits per second
    pub bandwidth_max: u64,
}
