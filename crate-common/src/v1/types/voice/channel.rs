use lamprey_macros::record;

use crate::v1::types::{ChannelId, UserId, voice::CallMetadata};

/// channel metadata for a voice channel
#[record]
pub struct ChannelVoice {
    /// bitrate for audio tracks. defaults to 65535 (64Kibps).
    #[validate(range(min = 8192))]
    pub bitrate: u64,

    /// maximum number of users who can be in this voice channel
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 100))]
    pub user_limit: Option<u64>,

    /// any currently active call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call: Option<CallMetadata>,

    // TODO: discord has these, unsure if i want to add these too
    #[cfg(any())]
    pub region: Option<String>,

    #[cfg(any())]
    pub video_quality: Option<u64>,
    // maybe use TrackEncoding for video quality? max_video_quality or video_quality_limit?
}

/// channel metadata for a broadcast channel
#[record]
pub struct ChannelBroadcast {
    /// the user this channel belongs to
    ///
    /// connecting clients should attempt to automatically focus this user's stream if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcaster_id: Option<UserId>,

    /// the stream schedule
    ///
    /// this should point to a calendar channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_id: Option<ChannelId>,
}
