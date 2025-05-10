// TODO: either copy thread/voice.rs to this or copy this to thread/voice.rs
// TODO: standardize terminology - everything is pretty loose right now

// current model:
// voice threads can have an associated call. calls have voicemembers. sfus
// exist in servers and regions

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, UserId};

use super::{CallId, LivestreamId, RoomId, ServerId, ThreadId};

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

/// a region which may contain multiple servers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Region {
    /// unique id for this region. usable as a translation key.
    pub id: String,

    /// this is the lowest latency region for the user (but not necessarily everyone!)
    pub optimal: bool,

    /// what this region can be used for right now
    pub availability: Availability,
}

/// a single server
///
/// (maybe rename to worker? if i implement federation, this could become pretty confusing...)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Server {
    /// an unique identifier for this server
    pub id: ServerId,

    /// this server's message of the day
    pub motd: Option<String>,

    // /// websocket endpoint; always uses the wss:// protocol
    // /// supports all standard query params: version, compress, format
    // pub endpoint: Url,
    /// this is the lowest latency server for the user (but not necessarily everyone!)
    pub optimal: bool,

    /// what this server can be used for right now
    pub availability: Availability,

    /// who's running the server. if None, this is an official server.
    pub hoster_id: Option<UserId>,
}

/// what this server or region can do
/// if 0, this likely can't be connected to at all
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Availability {
    /// a webrtc selective forwarding unit
    Sfu = 1 << 0,
}

// pub enum AvailabilityV2 {
//     /// a webrtc selective forwarding unit for voice and audio
//     Voice = 1 << 0,

//     /// a webrtc selective forwarding unit for video and streams
//     Video = 1 << 0,
//     Livestream = 1 << 0,
// }

/// ask for a specific region or server
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum HostOverride {
    /// use some server from this region
    Region(Region),

    /// use this specific server
    Server(Server),
}

/// a currently active call. sometimes called a "voice instance" in older docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceCall {
    pub id: CallId,
    pub thread_id: ThreadId,
    pub created_at: Time,

    /// current topic of the call
    pub topic: Option<String>,

    /// if call type is Broadcast, this only has the speakers
    pub members: Vec<VoiceMember>,

    #[serde(flatten)]
    pub info: CallType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum CallType {
    /// one to one or few to few
    ///
    /// (this should probably notify/ring the receiver)
    Direct,

    /// many to many
    Group,

    /// few to many
    Broadcast { audience_count: u64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceMember {
    pub user_id: UserId,
    pub room_id: RoomId,
    pub thread_id: ThreadId,
    pub call_id: CallId,

    /// when this person joined the call
    pub joined_at: Time,

    /// if this person is (self) deafened - can't hear anything
    pub deaf: bool,

    /// if this person has voice - if false, they're (self) muted
    pub voice: bool,

    /// if this person has their camera on
    pub camera: bool,

    /// if this person is livestreaming
    pub livestream: Option<Livestream>,

    /// the volume you set for this user. not shared; local to you.
    /// the volume of sound sent into your ears
    pub volume: Option<f64>,

    /// the time when someone requested to speak
    pub requested_voice_at: Option<Time>,
}

// if i move stuff perms into voice member/states
// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct VoiceMemberV2 {
//     pub user_id: UserId,
//     pub room_id: RoomId,
//     pub thread_id: ThreadId,
//     pub call_id: CallId,
//     pub session_id: (),

//     pub joined_at: Time,
//     pub deaf: bool,
//     pub mute: bool,
//     pub self_deaf: bool,
//     pub self_mute: bool,

//     // pub self_video: bool,
//     // pub self_stream: bool,
//     pub video: Vec<()>, // includes user and display media

//     pub suppress: bool,
//     pub requested_to_speak_at: Option<Time>,
// }

/// information about a livestream (screen sharing)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Livestream {
    started_at: Time,
    // need some cdn url for viewing a livestream
    livestream_id: LivestreamId,
    // // i know i should probably use the media api for this... but do i really need that entire system?
    // thumbnail_url: Option<Url>,
    // thumbnail_width: Option<u64>,
    // thumbnail_height: Option<u64>,
}
