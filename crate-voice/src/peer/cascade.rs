// TODO

use common::v1::types::SfuId;

struct PeerCascading {
    sfu_id: SfuId,
}

struct PeerCascadingInner {
    sfu_id: SfuId,
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
        // let quic = quinn::Endpoint::client(addr).unwrap();
        // let incoming_conn = quic.accept().await.unwrap();
        // let conn = incoming_conn.await.unwrap();
        // println!(
        //     "[server] connection accepted: addr={}",
        //     conn.remote_address()
        // );
        // conn.open_uni();
        // // Dropping all handles associated with a connection implicitly closes it
        todo!()
    }
}

impl Peer for PeerCascading {
    // TODO
}
