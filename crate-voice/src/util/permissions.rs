use common::v1::types::voice::internal::SfuVoiceState;

/// what a user can do in a voice channel/call
#[derive(Debug)]
pub struct Permissions {
    /// controls sending camera video, screenshare video, and screenshare audio
    pub video: bool,

    /// controls sending microphone
    pub audio: bool,

    /// controls receiving all audio tracks
    pub deaf: bool,
}

impl Permissions {
    pub fn all() -> Self {
        Self {
            video: true,
            audio: true,
            deaf: false,
        }
    }

    pub fn from_state(vs: &SfuVoiceState) -> Self {
        Self {
            video: vs.can_screenshare(),
            audio: vs.can_speak(),
            deaf: vs.is_deaf(),
        }
    }
}
