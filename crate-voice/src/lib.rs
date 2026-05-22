use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

use common::v1::types::{
    voice::{
        messages::SignallingCommand, MediaKind, SfuPermissions, SignallingMessage, Speaking,
        TrackKey, TrackMetadataWithPeerId, VoiceState,
    },
    ChannelId, UserId,
};
use str0m::{
    format::PayloadParams,
    media::{KeyframeRequestKind, MediaKind as MediaKindStr0m, MediaTime, Mid, Rid},
};

pub mod backbone;
pub mod backend;
pub mod error;
pub mod peer;
pub mod sfu;
pub mod signalling;
pub mod util;

pub use error::Error;

// /// a peer event with user_id, so the sfu knows where the event came from
// #[derive(Debug)]
// pub struct PeerEventEnvelope {
//     pub user_id: UserId,
//     pub payload: PeerEvent,
// }

// #[derive(Debug)]
// pub struct TrackIn {
//     pub kind: MediaKindStr0m,
//     pub state: TrackState,
//     pub channel_id: ChannelId,
//     pub key: TrackKey,
// }

// #[derive(Debug)]
// pub struct TrackOut {
//     pub kind: MediaKindStr0m,
//     pub state: TrackState,
//     pub peer_id: UserId,
//     pub source_mid: Mid,
//     pub enabled: bool,
//     pub channel_id: ChannelId,
//     pub key: TrackKey,
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum TrackState {
//     Pending,
//     Negotiating(Mid),
//     Open(Mid),
// }

// impl TrackState {
//     pub fn mid(&self) -> Option<Mid> {
//         match self {
//             TrackState::Pending => None,
//             TrackState::Negotiating(mid) => Some(*mid),
//             TrackState::Open(mid) => Some(*mid),
//         }
//     }
// }
