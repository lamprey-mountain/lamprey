//! sending and receiving media to peers

use std::future::Future;

use common::v1::types::{
    voice::{
        internal::MediaData,
        messages::{PeerEvent, SignallingCommand},
        KeyframeRequestKind, Mid, Rid, SpeakingWithPeerId, TrackMetadataWithPeerId,
    },
    PeerId,
};

pub mod cascade;
pub mod webrtc;
pub mod webrtc_old;

pub use webrtc_old::PeerWebrtc;

pub trait Peer {
    /// the unique id of this peer
    fn id(&self) -> PeerId;

    /// handle a command
    fn handle_command(&self, cmd: Command);

    /// another peer sent media data
    fn handle_media_data(&self, media: MediaData);

    /// another peer sent speaking metadata
    fn handle_speaking(&self, speaking: SpeakingWithPeerId);

    /// poll for events
    fn poll(&self) -> impl Future<Output = Option<PeerEvent>>;
}

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
    },

    /// another peer created a media track
    MediaAdded(TrackMetadataWithPeerId),
}
