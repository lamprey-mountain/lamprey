// TODO

use common::v1::types::{PeerId, SfuId};

use crate::peer::{Command, Peer};

struct PeerCascading {
    id: PeerId,
    // sfu_id: SfuId,
}

struct PeerCascadingInner {
    id: PeerId,
    // sfu_id: SfuId,
    // quic_conn: quin::Connection,
    // // This peer might be subscribed to MANY users' tracks
    // subscribed_tracks: HashSet<TrackKey>,
}

impl PeerCascading {
    pub fn spawn() -> Self {
        todo!()
    }
}

impl PeerCascadingInner {
    pub fn spawn() -> Self {
        todo!()
    }
}

impl Peer for PeerCascading {
    // /// the unique id of this peer
    // fn id(&self) -> PeerId;

    /// handle a command
    fn handle_command(&self, cmd: Command) {
        todo!()
    }

    // /// another peer sent media data
    // fn handle_media_data(&self, media: MediaData);

    // /// another peer sent speaking metadata
    // fn handle_speaking(&self, speaking: SpeakingWithPeerId);

    // /// poll for events
    // async fn poll(&self) -> Option<PeerEvent>;
}
