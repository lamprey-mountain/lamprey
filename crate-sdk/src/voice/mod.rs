use common::v2::types::ChannelId;

use crate::{Client, voice::client::VoiceBuilder};

pub mod client;
pub mod datachannel;
pub mod error;
pub mod player;
// pub mod player_old; // TODO: remove
pub mod track;

pub use error::VoiceError;

pub enum VoiceEvent {
    /// voice connection state changed
    StateChanged(VoiceConnectionStatus),

    // /// a user is speaking
    // UserSpeaking(SpeakingWithUserId),
    /// the voice client has been disconnected
    Disconnected,

    /// an error occured
    Error(VoiceError),
}

pub enum VoiceConnectionStatus {
    /// disconnected
    Disconnected,

    /// sent a VoiceState update, waiting for an sfu to connect to
    AwaitingSfu,

    /// connecting to the sfu
    Connecting,

    /// connected to the sfu
    Connected,

    /// no route to host
    NoRoute,

    /// webrtc ice checking
    IceChecking,
}

impl Client {
    /// create a voice connection
    pub fn voice(&self, channel_id: ChannelId) -> VoiceBuilder<'_> {
        VoiceBuilder::new(self, channel_id)
    }
}

// async fn example() {
//     let client: Client = todo!();
//     let vc = client.voice().channel(channel_id).connect().await?;
//     let audio = AudioFile::new_from_path("./path/to/media")?;
//     let audio = AudioTransform::new(audio);
//     let handle = audio.handle();
//     handle.set_volume(0.5);
//     vc.create_audio(audio).await?;
//     // now what?
// }
