use std::collections::HashMap;

use common::v1::types::{
    voice::{
        messages::{PeerCommand, SfuCommand},
        Mid, VoiceState,
    },
    PeerId,
};
use str0m::Rtc;

use crate::peer::{Command, Peer};

/// a handle to a webrtc peer connection
#[derive(Debug)]
pub struct PeerWebrtc {
    // ...?
}

// TODO: consider splitting incoming/outgoing rtc like livekit?
// apparently this prevents glare and has other nice things
#[derive(Debug)]
pub struct PeerWebrtcInner {
    /// rtc instance for incoming media
    rtc_incoming: Box<Rtc>,

    /// rtc instance for outgoing media
    rtc_outgoing: Box<Rtc>,

    voice_state: VoiceState,

    remote_mid_to_local_mid: HashMap<(PeerId, Mid), Mid>,
    // state: Arc<State>,
}

impl PeerWebrtc {
    pub fn spawn() -> Self {
        let inner = PeerWebrtcInner {
            rtc_incoming: todo!(),
            rtc_outgoing: todo!(),
            voice_state: todo!(),
            remote_mid_to_local_mid: todo!(),
        };

        tokio::spawn(async move {
            // write actual code here using `inner`
            todo!()
        });

        Self {}
    }
}

impl PeerWebrtcInner {
    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Signalling(_) => todo!(),
            Command::GenerateKeyframe { mid, rid, kind } => todo!(),
            Command::MediaAdded(track_metadata_with_peer_id) => todo!(),
        }
    }

    fn handle_local_media_added(&mut self, media_added: ()) {
        todo!()
    }

    fn handle_local_media_data(&mut self, media_data: ()) {
        todo!()
    }

    fn handle_local_media_speaking(&mut self, media_speaking: ()) {
        todo!()
    }

    fn handle_remote_media_added(&mut self, media_added: ()) {
        todo!()
    }

    fn handle_remote_media_data(&mut self, media_data: ()) {
        todo!()
    }

    fn handle_remote_media_speaking(&mut self, media_speaking: ()) {
        todo!()
    }
}

impl Peer for PeerWebrtc {
    fn id(&self) -> PeerId {
        todo!()
    }
}
