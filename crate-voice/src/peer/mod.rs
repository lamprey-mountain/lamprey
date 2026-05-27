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

#[async_trait]
pub trait Peer {
    /// the unique id of this peer
    fn id(&self) -> UserId;

    /// handle a command
    fn handle_command(&self, cmd: Command);

    /// another peer sent media data
    fn handle_media_data(&self, media: MediaData);

    /// another peer sent speaking metadata
    fn handle_speaking(&self, speaking: SpeakingWithUserId);

    /// poll for events
    async fn poll(&mut self) -> Option<PeerEvent>;
}

pub enum PeerEndpoint {
    Webrtc(webrtc::PeerWebrtc),
    Cascade(cascade::PeerCascading),
}

#[async_trait]
impl Peer for PeerEndpoint {
    fn id(&self) -> UserId {
        match self {
            PeerEndpoint::Webrtc(p) => p.id(),
            PeerEndpoint::Cascade(p) => p.id(),
        }
    }

    fn handle_command(&self, cmd: Command) {
        match self {
            PeerEndpoint::Webrtc(p) => p.handle_command(cmd),
            PeerEndpoint::Cascade(p) => p.handle_command(cmd),
        }
    }

    fn handle_media_data(&self, media: MediaData) {
        match self {
            PeerEndpoint::Webrtc(p) => p.handle_media_data(media),
            PeerEndpoint::Cascade(p) => p.handle_media_data(media),
        }
    }

    fn handle_speaking(&self, speaking: SpeakingWithUserId) {
        match self {
            PeerEndpoint::Webrtc(p) => p.handle_speaking(speaking),
            PeerEndpoint::Cascade(p) => p.handle_speaking(speaking),
        }
    }

    async fn poll(&mut self) -> Option<PeerEvent> {
        match self {
            PeerEndpoint::Webrtc(p) => p.poll().await,
            PeerEndpoint::Cascade(p) => p.poll().await,
        }
    }
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
    MediaAdded(TrackMetadataWithUserId),
    // /// peer limits updated
    // // TODO: handle channel bitrate
    // Limits { .. },
}

pub enum CommandFull {
    Inner(Command),
    MediaData(MediaData),
    Speaking(SpeakingWithUserId),
    NetworkPacket(SocketAddr, Bytes),
}
