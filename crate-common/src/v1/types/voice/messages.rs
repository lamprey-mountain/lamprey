//! messages between components of the voice system

use std::net::SocketAddr;

use lamprey_macros::record;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    ChannelId, SfuId, UserId,
    voice::{
        IceCandidate, SessionDescription, SfuStats, SubscriptionUpdate, TrackAnnouncement,
        TrackCreate, TrackId, TrackMapping, VoiceErrorCode, VoiceStateUpdate,
        internal::{SfuVoiceState, VoiceConfig},
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
        channel_id: ChannelId,
        state: SfuVoiceState,
    },

    /// replace a peer's voice state
    ///
    /// can be used to update permissions
    // TODO: add this
    #[cfg(any())]
    UpdatePeer {
        channel_id: ChannelId,
        state: SfuVoiceState,
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

    /// upsert channel config
    Channel { id: ChannelId, config: VoiceConfig },

    /// a remote peer wants a keyframe for this media
    // FIXME: keyframe generation between sfus
    // (this command specifically may not be needed)
    #[cfg(any())]
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
        tracks: Vec<TrackCreate>,
    },

    /// a sdp answer
    Answer { sdp: SessionDescription },

    /// an ice candidate
    Candidate { candidate: IceCandidate },

    /// update subscribed tracks
    Subscribe(SubscriptionUpdate),
}

/// an event sent from the backend to the peer's sync connection
#[record]
#[serde(tag = "type")]
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
        tracks: Vec<TrackMapping>,
    },

    /// a sdp answer
    Answer { sdp: SessionDescription },

    /// an ice candidate
    Candidate { candidate: IceCandidate },

    /// update available tracks for a user
    Tracks {
        user_id: UserId,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        added: Vec<TrackAnnouncement>,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        removed: Vec<TrackId>,
    },

    /// update subscribed tracks
    Subscribe(SubscriptionUpdate),

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
}
