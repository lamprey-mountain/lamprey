// do i have all signalling go through the main events websocket, or only
// do sdp/ice/connection and do signalling directly against the sfu with
// datachannels? i feel like the second could be nicer but harder.

use common::v1::types::{RtcPeerId, UserId};
use serde::{Deserialize, Serialize};
use str0m::{
    change::{SdpAnswer, SdpOffer},
    media::{MediaAdded, MediaData, Mid},
    Candidate,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RtcPeerCommand {
    /// sdp answer (via websocket)
    Answer { sdp: String },

    /// sdp offer (via websocket)
    Offer { sdp: String },

    /// ice candidate proposal (via websocket)
    IceCandidate {
        data: String,
        // candidate: Candidate,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RtcPeerEvent {
    /// sdp answer (via websocket)
    Answer { sdp: String },

    /// sdp offer (via websocket)
    Offer { sdp: String },
    // /// ice candidate proposal (via websocket)
    // IceCandidate { candidate: Candidate },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    /// the user who sent this, or None if this is from the server
    pub user_id: Option<UserId>,

    #[serde(flatten)]
    pub inner: RtcPeerCommand,
}
