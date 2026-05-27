use std::ops::Deref;

// TODO: add doc comments

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use uuid::Uuid;
#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    util::Time, ChannelId, ConnectionId, MediaId, RoomId, RoomMember, SessionId, User, UserId,
};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

/// a unique identifier for a media track (corresponds to a transceiver in webrtc)
///
/// media track ids are unique per peer connection (peer-peer pair)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mid(pub Uuid);

/// a unique identifier for a track layer
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Rid(pub u64);

impl Deref for SessionDescription {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for IceCandidate {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// represents a user that is connected to a voice channel
///
/// older docs call this a "voice connection"
///
/// ## connection limits
///
/// - Users can only have one voice state per channel
/// - Non-bots can only have one state across all channels in all rooms
/// - Bots can have any number of voice states
// TODO: add room_id?
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceState {
    /// the user this state belongs to
    pub user_id: UserId,

    /// the channel this user is connected to
    pub channel_id: ChannelId,

    /// the session that's being used to connect to this voice channel
    ///
    /// this is only be returned for the user this state belongs to
    pub session_id: Option<SessionId>,

    /// the sync connection that's being used
    ///
    /// this is only be returned for the user this state belongs to
    pub connection_id: Option<ConnectionId>,

    /// when this user joined the call
    pub joined_at: Time,

    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,

    /// whether this user has muted themselves
    pub self_mute: bool,

    /// whether this user has deafened themselves
    pub self_deaf: bool,

    /// whether this user has enabled their camera
    pub self_video: bool,

    /// populated if the user is sharing their screen
    pub screenshare: Option<VoiceStateScreenshare>,

    /// whether this user is suppressed, similar to a transient `mute: true`
    pub suppress: bool,

    /// when this user requested to speak
    pub requested_to_speak_at: Option<Time>,
    // TODO: positional audio
}

/// the voice state with user/room member info
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateFull {
    #[serde(flatten)]
    pub inner: VoiceState,
    pub user: User,
    pub member: Option<RoomMember>,
    // pub thread_member: Option<ThreadMember>,
}

// /// the current status of a voice state
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum VoiceStatus {
//     /// waiting for user to connect
//     Connecting,

//     /// connected and active
//     Connected,
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum VoiceStateFlags {
//     /// this user is muted by a moderator
//     ///
//     /// corresponds to `RoomMember`'s `mute` field
//     Mute,
//     Deaf,
//     SelfMute,
//     SelfDeaf,
//     HasVideo,

//     /// this user is suppressed
//     ///
//     /// similar to a transient `mute: true`. enabled by default in broadcast channels and the afk channel.
//     Suppressed,
// }

/// info about a user's screen share
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateScreenshare {
    /// when this user started sharing their screen
    pub started_at: Time,

    // /// the mid of the screenshare
    // pub mid: Mid,
    /// the thumbnail for the user's screenshare
    ///
    /// this is an image from the screenshare. should be updated periodically.
    pub thumbnail: Option<MediaId>,
    // /// the clip for the user's screenshare
    // ///
    // /// this is a short recording from the screenshare. should be updated periodically.
    // pub clip: Option<MediaId>,
}

/// represents an update that a user would like to make to their voice state
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateUpdate {
    pub channel_id: ChannelId,
    pub self_deaf: bool,
    pub self_mute: bool,

    // NOTE: disable manually updating this?
    pub self_video: bool,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub screenshare: Option<Option<VoiceStateScreenshareUpdate>>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateScreenshareUpdate {
    /// the thumbnail for the user's stream. should be updated periodically.
    pub thumbnail: Option<MediaId>,
}

/// metadata about a track
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackMetadata {
    /// unique identifier for this track
    ///
    /// equivalent to transceiver.mid
    pub mid: Mid,

    /// whether this track is for audio or video
    pub kind: MediaKind,

    /// group tracks together into streams; identical to ssrc but easier to manage client side
    ///
    /// currently there are two streams `user` and `screen` used by frontend
    pub key: TrackKey,

    /// simulcasting layers, only applicable for video
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub layers: Vec<TrackLayer>,
    // /// whisper config
    // pub whisper: Option<TrackWhisper>,
}

// // TODO: whispering
// /// whispering config (only send media from this track to these users)
// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct TrackWhisper {
//     pub user_ids: Vec<UserId>,
// }

/// track metadata. `mid` is the **mapped** media id, ie. the mid used between the final sfu/peer
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackMetadataWithUserId {
    /// the inner track metadata
    // NOTE: sometimes the `mid` field is the *source* mid, sometimes its the mapped mid
    // i should create types to represent this more strictly
    #[serde(flatten)]
    pub inner: TrackMetadata,

    /// the source user this track came from
    pub user_id: UserId,
}

#[cfg(any())]
mod next {
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "utoipa")]
    use utoipa::ToSchema;

    use crate::v1::types::{voice::Mid, UserId};

    use super::{MediaKind, TrackKey, TrackLayer};

    /// the base metadata for a track
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    pub struct TrackMetadataInner {
        /// whether this track is for audio or video
        pub kind: MediaKind,

        /// group tracks together into streams; identical to ssrc but easier to manage client side
        ///
        /// currently there are two streams `user` and `screen` used by frontend
        pub key: TrackKey,

        /// simulcasting layers, only applicable for video
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        pub layers: Vec<TrackLayer>,
        // /// whisper config
        // pub whisper: Option<TrackWhisper>,
    }

    /// metadata of the track at its origin point.
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    pub struct TrackMetadataSource {
        #[serde(flatten)]
        pub inner: TrackMetadataInner,

        /// the source mid
        pub source_mid: Mid,

        /// the source user this track came from
        pub user_id: UserId,
    }

    /// metadata of a track projected to a subscriber with a remapped media id.
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    pub struct TrackMetadataMapped {
        #[serde(flatten)]
        pub inner: TrackMetadataInner,

        /// the mapped mid
        pub mapped_mid: Mid,

        /// the source user this track came from
        pub user_id: UserId,
    }
}

/// which stream this track is associated with. generally there will be one video track and one audio track per stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackKey {
    /// media from the user (microphone, camera)
    User,

    /// a screenshare
    Screen,

    /// an unknown track type
    #[serde(untagged)]
    Other(String),
}

// TODO: doc comment
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackLayer {
    pub rid: Rid,
    pub encoding: TrackEncoding,
}

/// the encoding of the track
// TODO: explicitly specify bitrate, framerate, resolution?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackEncoding {
    /// source resolution
    Source,

    // /// reduced resolution
    // Reduced,
    /// reduced thumbnail resolution
    Thumbnail,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Subscription {
    /// which track to subscribe to
    pub mid: Mid,

    /// the layers of the track to subscribe to
    ///
    /// - clients should only subscribe to one layer at a time, but multiple can be subscribed if needed
    /// - the server may subscribe to multiple depending on if multiple resolutions are requested
    /// - leave empty for audio tracks
    #[cfg_attr(feature = "serde", serde(default))]
    pub rid: Vec<Rid>,
}

/// the kind of media this track is for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
// TODO: rename_all lowercase
pub enum MediaKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum KeyframeRequestKind {
    /// just joined a stream, needs a keyframe for initial rendering
    Fir,

    /// lost some data, need a keyframe to recover
    Pli,
}

/// Flags for speaking
///
/// Audio = 1 << 0
/// Indicator = 1 << 1
/// Priority = 1 << 2
// TODO: Broadcast = 1 << 3
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SpeakingFlags(pub u8);

impl SpeakingFlags {
    #[inline]
    pub fn has_audio(&self) -> bool {
        self.0 & 1 == 1
    }

    #[inline]
    pub fn has_indicator(&self) -> bool {
        self.0 & 2 == 2
    }

    #[inline]
    pub fn has_priority(&self) -> bool {
        self.0 & 4 == 4
    }
}

/// a message sent from the client to indicate that they're speaking (among other things)
// could be fun to add other filters? like lowpass, reverb, etc (can be done client side)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Speaking {
    pub mid: Mid,
    pub flags: SpeakingFlags,
}

/// a message sent to the client to indicate that someone is speaking
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SpeakingWithUserId {
    pub user_id: UserId,
    pub source_mid: Mid,
    pub flags: SpeakingFlags,
}

impl VoiceState {
    pub fn muted(&self) -> bool {
        self.mute || self.self_mute || self.suppress
    }

    pub fn deafened(&self) -> bool {
        self.deaf || self.self_deaf
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateMove {
    pub target_id: ChannelId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateMoveBulk {
    /// set to None to move everyone
    pub user_ids: Option<Vec<UserId>>,

    /// target channel id
    pub channel_id: ChannelId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStatePatch {
    /// allow this user to speak in the current channel
    ///
    /// requires VoiceMute permission
    pub suppress: Option<bool>,

    /// same as room member deaf
    pub deaf: Option<bool>,

    /// same as room member mute
    pub mute: Option<bool>,

    /// where to move this participant. you can only move participants to the channels in the same room.
    pub channel_id: Option<ChannelId>,

    /// when this user requested to speak
    ///
    /// - users can only set this for themselves
    /// - this can only be set to the current time
    /// - you must have VoiceRequest to set this
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub requested_to_speak_at: Option<Option<Time>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CallCreate {
    /// call topic
    ///
    /// must have VoiceMute permission in target channel to set
    // NOTE: unsure about using this permission
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 512)))]
    pub topic: Option<String>,
}

/// a currently active voice session
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Call {
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub topic: Option<String>,

    /// when this call was created
    ///
    /// roughly corresponds to the time that the first user joined
    pub created_at: Time,
    // /// how many people are in the audience
    // ///
    // /// only populated if this is a broadcast channel. in broadcast channels,
    // /// only voice states for yourself and speakers (ie. users who are not
    // /// suppressed) are sent.
    // // TODO: skip serializing if None
    // pub audience_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CallPatch {
    /// the current call topic
    ///
    /// only unsuppressed users can change the call topic
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 512)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub topic: Option<Option<String>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct CallDeleteParams {
    /// if people are still connected to this channel, try to forcibly disconnect them
    ///
    /// requires VoiceDisconnect permission
    #[cfg_attr(feature = "serde", serde(default))]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RingEligibility {
    /// whether ring endpoints can be used
    ///
    /// true in dms and gdms, false otherwise
    pub ringable: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RingStart {
    pub user_ids: Vec<UserId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RingStop {
    pub user_ids: Vec<UserId>,
}

// TODO: use for various voice_state_foo routes
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct VoiceStateParams {
    /// whether to return the full voice state
    #[serde(default)]
    pub full: bool,
}

/// channel metadata for a voice channel
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelVoice {
    /// bitrate, for voice channels. defaults to 65535 (64Kibps).
    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    pub bitrate: Option<u64>,

    /// maximum number of users who can be in this voice channel
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub user_limit: Option<u64>,
    // TODO: discord has these, unsure if i want to add these too
    // pub region: Option<String>,
    // pub video_quality: Option<u64>,
    // /// any currently active call
    // pub call: Option<Call>,
}

/// channel metadata for a broadcast channel
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelBroadcast {
    /// the user this channel belongs to
    ///
    /// connecting clients should attempt to automatically focus this user's stream if it exists
    pub broadcaster_id: Option<UserId>,

    /// the stream schedule
    ///
    /// this should point to a calendar channel
    pub schedule_id: Option<ChannelId>,
}
