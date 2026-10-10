//! types for keeping a local copy of state in sync

use lamprey_macros::record;

use crate::v1::types::MessageSync;

pub mod seq;

pub use seq::{ChannelSeq, RoomSeq};

/// Response from the channel mirror endpoint.
///
/// Contains incremental sync events to apply to local state.
#[record]
pub struct ChannelSync {
    /// sync events to apply to local state
    pub events: Vec<MessageSync>,
    // pub events: Vec<DispatchChannelInner>, // v2
    /// the new latest sequence number you have
    pub seq: ChannelSeq,

    /// not all events were returned. call this endpoint again with the new `seq`
    pub partial: bool,
}

// /// response for the room sync endpoint
// ///
// /// contains incremental sync events to apply to local state
// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct RoomMirrorUpdate {
//     /// sync events to apply to local state
//     pub events: Vec<DispatchRoomInner>,

//     /// the new latest sequence number you have
//     pub seq: RoomSeq,

//     /// not all events were returned. call this endpoint again with the new `seq`
//     pub partial: bool,
// }

// TODO: also have user sync?
