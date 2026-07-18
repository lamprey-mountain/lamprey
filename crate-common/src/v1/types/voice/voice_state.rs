use lamprey_macros::record;

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v1::types::{
    ChannelId, ConnectionId, MediaId, RoomId, RoomMember, SessionId, ThreadMember, User, UserId,
    misc::Time,
};

/// represents a user that is connected to a voice channel
///
/// older docs call this a "voice connection"
///
/// ## connection limits
///
/// - Users can only have one voice state per channel
/// - Non-bots can only have one state across all channels in all rooms
/// - Bots can have any number of voice states
// TODO: maybe rename this back to VoiceConnection, VoiceSession, etc?
#[record]
pub struct VoiceState {
    /// the user this state belongs to
    pub user_id: UserId,

    /// the channel this user is connected to
    pub channel_id: ChannelId,

    /// the room this user is connected to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    /// the session that's being used to connect to this voice channel
    ///
    /// this is only be returned for the user this state belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,

    /// the sync connection that's being used
    ///
    /// this is only be returned for the user this state belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshare: Option<VoiceStateScreenshare>,

    /// whether this user is suppressed, similar to a transient `mute: true`
    pub suppress: bool,

    /// when this user requested to speak
    pub requested_to_speak_at: Option<Time>,
}

/// the voice state with user/room member info
#[record]
pub struct VoiceStateFull {
    #[serde(flatten)]
    pub inner: VoiceState,

    pub user: User,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<RoomMember>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_member: Option<ThreadMember>,
}

/// represents an update to a voice state
///
/// sent via rest
#[record]
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
    #[serde(default, deserialize_with = "some_option")]
    pub requested_to_speak_at: Option<Option<Time>>,
}

/// info about a user's screen share
#[record]
pub struct VoiceStateScreenshare {
    /// when this user started sharing their screen
    pub started_at: Time,

    /// the thumbnail for the user's screenshare
    ///
    /// this is an image from the screenshare. should be updated periodically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<MediaId>,

    // TODO: implement
    /// the clip for the user's screenshare
    ///
    /// this is a short recording from the screenshare. should be updated periodically.
    #[cfg(any())]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<MediaId>,
}

/// represents an update that a user would like to make to their voice state
#[record]
pub struct VoiceStateUpdate {
    pub channel_id: ChannelId,
    pub self_deaf: bool,
    pub self_mute: bool,

    // NOTE: disable manually updating this?
    pub self_video: bool,

    #[serde(default, deserialize_with = "some_option")]
    pub screenshare: Option<Option<VoiceStateScreenshareUpdate>>,
}

#[record]
pub struct VoiceStateScreenshareUpdate {
    /// the thumbnail for the user's stream. should be updated periodically.
    pub thumbnail: Option<MediaId>,
}

// TODO: use for various voice_state_foo routes
#[record]
pub struct VoiceStateParams {
    /// whether to return the full voice state
    #[serde(default)]
    pub full: bool,
}

#[record]
pub struct VoiceStateMove {
    pub target_id: ChannelId,
}

#[record]
pub struct VoiceStateMoveBulk {
    /// set to None to move everyone
    pub user_ids: Option<Vec<UserId>>,

    /// target channel id
    pub channel_id: ChannelId,
}

impl VoiceState {
    pub fn muted(&self) -> bool {
        self.mute || self.self_mute || self.suppress
    }

    pub fn deafened(&self) -> bool {
        self.deaf || self.self_deaf
    }

    pub fn apply_update(&mut self, update: VoiceStateUpdate) {
        self.channel_id = update.channel_id;
        self.self_deaf = update.self_deaf;
        self.self_mute = update.self_mute;
        self.self_video = update.self_video;
        if let Some(screenshare) = update.screenshare {
            self.screenshare = screenshare.map(|s| VoiceStateScreenshare {
                started_at: Time::now_utc(),
                thumbnail: s.thumbnail,
            });
        }
    }

    pub fn apply_patch(&mut self, patch: VoiceStatePatch) {
        if let Some(suppress) = patch.suppress {
            self.suppress = suppress;
        }
        if let Some(deaf) = patch.deaf {
            self.deaf = deaf;
        }
        if let Some(mute) = patch.mute {
            self.mute = mute;
        }
        if let Some(channel_id) = patch.channel_id {
            self.channel_id = channel_id;
        }
        if let Some(requested_to_speak_at) = patch.requested_to_speak_at {
            self.requested_to_speak_at = requested_to_speak_at;
        }
    }
}
