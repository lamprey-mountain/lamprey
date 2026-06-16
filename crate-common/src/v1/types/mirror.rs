//! types for keeping a local copy of state in sync

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::MessageSync;

/// A monotonic sync token, incremented on every action in a channel.
///
/// Used for incremental sync to determine what events the client is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelSeq(pub u64); // TEMP: pub

impl ChannelSeq {
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for ChannelSeq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for ChannelSeq {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<ChannelSeq> for u64 {
    fn from(value: ChannelSeq) -> Self {
        value.0
    }
}

/// A monotonic sync token, incremented on every action in a room.
///
/// Used for incremental sync to determine what events the client is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomSeq(pub u64);

impl RoomSeq {
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for RoomSeq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for RoomSeq {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<RoomSeq> for u64 {
    fn from(value: RoomSeq) -> Self {
        value.0
    }
}

/// Response from the channel mirror endpoint.
///
/// Contains incremental sync events to apply to local state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelMirror {
    /// sync events to apply to local state
    pub events: Vec<MessageSync>,
    // pub events: Vec<DispatchChannelInner>, // v2
    /// the new latest sequence number you have
    pub seq: ChannelSeq,

    /// not all events were returned. call this endpoint again with the new `seq`
    pub partial: bool,
}

// TEMP: add alias backwards compat
pub use ChannelMirror as ChannelSync;

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
