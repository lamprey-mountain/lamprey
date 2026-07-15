//! messages between components of the voice system

// TODO: i need some way to be able to update a user's permissions

use std::net::SocketAddr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    ChannelId, SfuId, UserId,
    voice::{
        IceCandidate, KeyframeRequestKind, Mid, Rid, SessionDescription, Speaking,
        SpeakingWithUserId, Subscription, TrackMetadata, TrackMetadataWithUserId, VoiceErrorCode,
        VoiceState, VoiceStateUpdate,
        internal::{SfuChannel, SfuPermissions, SfuStats},
    },
};

/// a command the master uses to control the sfu
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
pub enum SfuCommand {
    /// initial config for this sfu
    Init {
        /// the id of this sfu
        sfu_id: SfuId,
    },

    /// should recalculate latency
    RecalculateLatency { target_sfu: SfuId },

    /// move these peers to another sfu
    MigrateUsers {
        users: Vec<UserId>,
        target_sfu: SfuId,
    },

    /// create a new peer
    ///
    /// sends `PeerCreated` when ready
    CreatePeer {
        state: VoiceState,
        permissions: SfuPermissions,
    },

    /// create a new cascading connection
    ///
    /// sends `CascadeCreated` after connecting to the sfu
    CreateCascade {
        /// the id of the target sfu
        sfu_id: SfuId,

        /// the secret token to authentication with
        token: String,

        /// the address to connect to
        addr: SocketAddr,
    },

    /// whenever media is sent to this channel, forward it to all of these sfus
    RouteUpdate {
        channel_id: ChannelId,
        destinations: Vec<SfuId>,
    },

    /// proxied signalling message from a user
    Signalling {
        /// the user who sent this
        user_id: UserId,

        /// the channel they sent this for
        channel_id: ChannelId,

        inner: SignallingCommand,
    },

    /// upsert channel
    Channel { channel: SfuChannel },

    /// a remote peer wants a keyframe for this media
    GenerateKeyframe {
        /// the track to generate a keyframe for
        mid: Mid,

        /// the rid to generate a keyframe for
        rid: Option<Rid>,

        /// the kind of the keyframe that should be generated
        kind: KeyframeRequestKind,

        /// the id of the peer that requested the keyframe
        user_id: UserId,
    },
}

/// an event emitted by the sfu for the master
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
pub enum SfuEvent {
    /// calculated rtt latency to another sfu
    ///
    /// response to `SfuCommand::RecalculateLatency`, but can also be sent whenever the sfu feels like
    Latency {
        target_sfu: SfuId,

        /// round trip time in nanoseconds
        rtt: u32,
    },

    /// stats for this sfu
    Stats { stats: SfuStats },

    /// a peer has been created
    PeerCreated {
        user_id: UserId,
        channel_id: ChannelId,
    },

    /// a peer has disconnected
    // TODO: rename to PeerDisconnected
    PeerDisconnect {
        user_id: UserId,
        channel_id: ChannelId,
    },

    /// a cascade has been created
    CascadeCreated {
        sfu_id: SfuId,
        channel_id: ChannelId,
    },

    /// a cascade has been prepared
    ///
    /// contains info needed to connect to another sfu
    CascadePrepared {
        /// the id of the connecting sfu
        sfu_id: SfuId,

        /// the secret token to authenticate with
        token: String,

        /// the address to connect to
        addr: SocketAddr,
    },

    // TODO: CascadeDisconnected
    /// send this message to this user
    VoiceDispatch {
        user_id: UserId,
        channel_id: ChannelId,
        payload: Box<SignallingEvent>,
    },

    /// update the voice state of a peer
    VoiceState {
        user_id: UserId,
        channel_id: ChannelId,
        update: VoiceStateUpdate,
    },
}

/// an event sent from the peer's sync connection to the master
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SignallingCommand {
    /// disconnect
    Disconnect,

    /// update client's voice state
    VoiceState { state: VoiceStateUpdate },

    /// a sdp offer
    Offer {
        sdp: SessionDescription,
        tracks: Vec<TrackMetadata>,
    },

    /// a sdp answer
    Answer { sdp: SessionDescription },

    /// an ice candidate
    Candidate { candidate: IceCandidate },

    /// request additional tracks
    ///
    /// - all audio from key `user` is sent by default
    /// - all video and audio from other sources require a subscription
    /// - sent by server and client
    /// - replaces the previous subscription
    Subscribe { subs: Vec<Subscription> },
}

/// an event sent from the backend to the peer's sync connection
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SignallingEvent {
    /// the sfu is ready to accept voice payloads
    Connected {
        /// the id of the selected sfu
        ///
        /// internal; for debugging.
        sfu_id: SfuId,
    },

    /// disconnected
    Disconnected,

    /// a sdp offer
    Offer {
        sdp: SessionDescription,
        tracks: Vec<TrackMetadataWithUserId>,
    },

    /// a sdp answer
    Answer { sdp: SessionDescription },

    /// an ice candidate
    Candidate { candidate: IceCandidate },

    /// update (replace) available tracks for a user
    Tracks {
        user_id: UserId,
        tracks: Vec<TrackMetadata>,
    },

    /// request additional tracks
    ///
    /// - all audio from key `user` is sent by default
    /// - all video and audio from other sources require a subscription
    /// - sent by server and client
    /// - replaces the previous subscription
    Subscribe { subs: Vec<Subscription> },

    /// migrate to a new sfu
    ///
    /// assume a new connection has been created for you with your existing `VoiceState`, you need to negotiate with a new rtc peer connection then destoy the old one.
    Migrate { new_sfu_id: SfuId },

    /// an error has occured
    Error {
        /// human readable error message
        message: String,

        /// what exactly went wrong
        code: VoiceErrorCode,
    },
    // /// please send a thumbnail for the current stream
    // WantThumbnail,
    // /// a user connected to the call
    // Connected { user_id: UserId },

    // /// a user disconnected to the call
    // Disconnected { user_id: UserId },

    // /// response to Subscribe
    // Subscribed {
    //     voice: Vec<UserId>,
    //     camera: Vec<UserId>,
    //     livestream: Vec<UserId>,
    // },
}

/// a message from one sfu host to another
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum BackboneDispatch {
    /// acknowledge a dispatch
    Ack,

    /// sent by client on connect
    Hello {
        /// auth token
        token: String,
    },

    /// cleanly disconnect
    Disconnect,

    /// a peer needs a keyframe to render
    Keyframe {
        /// the id of the user the track is from
        user_id: UserId,

        /// the track to generate a keyframe for
        mid: Mid,

        /// the rid to generate a keyframe for
        rid: Option<Rid>,

        /// the kind of the keyframe that should be generated
        kind: KeyframeRequestKind,
    },

    // TODO: use this instead of {Track,Subscription}{Create,Remove}
    // Signalling(SignallingCommand),
    /// sender sfu has these tracks
    TrackCreate {
        channel_id: ChannelId,
        tracks: Vec<TrackMetadataWithUserId>,
    },

    /// sender sfu no longer has these tracks
    TrackRemove {
        channel_id: ChannelId,
        tracks: Vec<(Mid, UserId)>,
    },

    /// sender sfu wants these tracks
    SubscriptionCreate {
        channel_id: ChannelId,
        subscriptions: Vec<(Subscription, UserId)>,
    },

    /// sender sfu no longer wants these tracks
    SubscriptionRemove {
        channel_id: ChannelId,
        subscriptions: Vec<(Mid, UserId)>,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct BackboneDispatchEnvelope {
    pub nonce: Option<String>,
    pub dispatch: BackboneDispatch,
}

// TODO: remove?
// /// a datagram sent between sfu hosts
// #[derive(Debug, Clone)]
// pub enum BackboneDatagram {
//     Media(MediaData),
//     Speaking(SpeakingWithUserId),
// }

// #[derive(Debug, thiserror::Error)]
// pub enum BackboneDatagramDeserializeError {
//     /// payload is empty
//     #[error("payload is empty")]
//     EmptyPayload,

//     /// payload unexpectedly ended
//     #[error("payload unexpectedly ended")]
//     UnexpectedEof,

//     /// unknown payload type
//     #[error("unknown payload type: {0}")]
//     UnknownPayloadType(u8),
// }

// impl BackboneDatagram {
//     /// serialize this datagram to bytes
//     pub fn to_bytes(&self) -> Bytes {
//         let mut buf = Vec::new();
//         use bytes::BufMut;
//         match self {
//             BackboneDatagram::Media(m) => {
//                 buf.put_u8(0);
//                 buf.put_slice(&m.to_bytes());
//             }
//             BackboneDatagram::Speaking(s) => {
//                 buf.put_u8(1);
//                 buf.put_slice(s.user_id.as_bytes());
//                 buf.put_slice(&s.mid.0);
//                 buf.put_u8(s.flags.0);
//             }
//         }
//         buf.into()
//     }

//     /// deserialize this datagram from bytes
//     pub fn from_bytes(bytes: &[u8]) -> Result<Self, BackboneDatagramDeserializeError> {
//         if bytes.is_empty() {
//             return Err(BackboneDatagramDeserializeError::EmptyPayload);
//         }
//         let tag = bytes[0];
//         let payload = &bytes[1..];
//         match tag {
//             0 => {
//                 let m = MediaData::from_bytes(payload)
//                     .map_err(|_| BackboneDatagramDeserializeError::UnexpectedEof)?;
//                 Ok(BackboneDatagram::Media(m))
//             }
//             1 => {
//                 use bytes::Buf;
//                 let mut buf = payload;
//                 if buf.remaining() < 16 + 16 + 1 {
//                     return Err(BackboneDatagramDeserializeError::UnexpectedEof);
//                 }
//                 let mut peer_bytes = [0u8; 16];
//                 buf.copy_to_slice(&mut peer_bytes);
//                 let user_id = UserId::from(Uuid::from_bytes(peer_bytes));

//                 let mut mid_bytes = [0u8; 16];
//                 buf.copy_to_slice(&mut mid_bytes);
//                 let source_mid = Mid(mid_bytes);

//                 let flags = SpeakingFlags(buf.get_u8());

//                 Ok(BackboneDatagram::Speaking(SpeakingWithUserId {
//                     user_id,
//                     mid: source_mid,
//                     flags,
//                 }))
//             }
//             _ => Err(BackboneDatagramDeserializeError::UnknownPayloadType(tag)),
//         }
//     }
// }
