//! sending and receiving media to peers

use std::net::SocketAddr;

use async_trait::async_trait;
use bytes::Bytes;
use common::v1::types::{
    voice::{
        internal::MediaData,
        messages::{PeerEvent, SignallingCommand},
        KeyframeRequestKind, Mid, Rid, SpeakingWithUserId, TrackMetadataWithUserId,
    },
    UserId,
};

pub mod cascade;
pub mod webrtc;

pub enum Command {
    /// proxied signalling message from a peer
    Signalling(SignallingCommand),

    /// a remote peer wants a keyframe for this media
    GenerateKeyframe {
        /// the track to generate a keyframe for
        mid: Mid,

        /// the rid to generate a keyframe for
        rid: Option<Rid>,

        /// the kind of the keyframe that should be generated
        kind: KeyframeRequestKind,

        /// the id of the user that requested the keyframe
        user_id: UserId,
    },

    /// another peer created a media track
    MediaAdded(TrackMetadataWithUserId),
    // /// peer limits updated
    // // TODO: handle channel bitrate
    // Limits { .. },
}

// NOTE: is there really any reason for these variants of CommandFull to be split out from Command?
// TODO: merge this into Command...? or create multiple `enum Command`s
pub enum CommandFull {
    Inner(Command),
    MediaData(MediaData),
    Speaking(SpeakingWithUserId),
    NetworkPacket(SocketAddr, Bytes),
}
