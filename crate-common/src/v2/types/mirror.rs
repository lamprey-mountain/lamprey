// TODO: promote to v1?

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{v1::types::ChannelSeq, v2::types::sync::channel::DispatchChannelInner};

/// response for the channel sync endpoint
///
/// contains incremental sync events to apply to local state
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelMirrorUpdate {
    /// sync events to apply to local state
    pub events: Vec<DispatchChannelInner>,

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

// TODO: maybe also have user sync?
