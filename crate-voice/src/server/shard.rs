use std::{collections::HashMap, net::SocketAddr};

use bytes::{Bytes, BytesMut};
use slotmap::SlotMap;
use tokio::{net::UdpSocket, sync::mpsc};

use crate::prelude::*;
use crate::util::stun::extract_local_ufrag;
use crate::{backend::BackendHandle, server::shard_call::ShardCall, util::SfuVoiceState};

// one shard per thread
pub struct Shard {
    backend: BackendHandle,
    control_rx: mpsc::Receiver<ShardCommand>,

    sock_v4: UdpSocket,
    sock_v6: UdpSocket,

    calls: SlotMap<CallSlot, ShardCall>,
    // event_queue: Vec<SfuEvent>,
    // events: VecDequeue<SfuEvent>,
    /// map of socket addresses to peer (after ice nomination)
    addrs: HashMap<SocketAddr, (CallSlot, PeerSlot)>,

    /// map of ufrags to peers (before ice nomination)
    ufrags: HashMap<String, (CallSlot, PeerSlot)>,
}

pub struct ShardHandle {
    control_tx: mpsc::Sender<ShardCommand>,
}

enum ShardCommand {
    // /// create a new peer
    // CreatePeer(SfuVoiceState),
    // // /// a signalling command that the user sent
    // // Signalling {
    // //     user_id: UserId,
    // //     inner: SignallingCommand,
    // // },

    // // GenerateKeyframe {
    // //     user_id: UserId,
    // //     mid: Mid,
    // //     rid: Option<Rid>,
    // //     kind: KeyframeRequestKind,
    // // },
}

// enum ShardEvent {
//     // TODO
// }

impl Shard {
    // one shared udp socket (per shard or per core?)
    // "demux via ICE ufrag in STUN binding requests"

    async fn run(&mut self) {
        let mut buf_v4 = BytesMut::with_capacity(2000);
        let mut buf_v6 = BytesMut::with_capacity(2000);

        loop {
            // cleanup dead peers
            // process sdp renegotiation
            // drain events/send stuff to peers

            tokio::select! {
                Ok((len, source)) = self.sock_v4.recv_from(&mut buf_v4) => {
                    todo!()
                }

                Ok((len, source)) = self.sock_v6.recv_from(&mut buf_v6) => {
                    todo!()
                }

                Some(cmd) = self.control_rx.recv() => {
                    // self.handle_command(cmd).await;
                    todo!()
                }

                // _ = sleep_future => {
                //     self.drive_timers().await;
                // }
            }
        }
    }

    /// handle a udp packet from `dst` to `src` with data `data`
    fn handle_udp(&mut self, dst: SocketAddr, src: SocketAddr, data: Bytes) {
        let now = std::time::Instant::now();
        let input = SInput::Receive(
            now,
            str0m::net::Receive {
                proto: str0m::net::Protocol::Udp,
                source: src,
                destination: dst,
                contents: data
                    .as_ref()
                    .try_into()
                    .expect("TODO: better error handling"),
            },
        );

        // find the destination peer
        let peer_loc = if let Some(&loc) = self.addrs.get(&src) {
            Some(loc)
        } else if let Some(local_ufrag) = extract_local_ufrag(&data) {
            if let Some(&loc) = self.ufrags.get(&local_ufrag) {
                self.addrs.insert(src, loc);
                Some(loc)
            } else {
                None
            }
        } else {
            None
        };

        let Some((call, peer)) = peer_loc else {
            // TODO: warn/debug/trace log for unresolvable packets?
            return;
        };

        if let Some(call) = self.calls.get_mut(call) {
            call.handle_input(peer, input);
        }
    }

    // fn handle_disconnect(&mut self, peer_id: PeerId) {
    // async fn handle_peer_event(&mut self, peer_id: PeerId, event: Event) {}

    // /// handle a shard command
    // fn handle_command(&mut self, cmd: ShardCommand) {}
}

impl ShardHandle {
    pub fn create_peer(&self, s: SfuVoiceState) {
        todo!()
    }

    // pub fn generate_keyframe(&self, ...) {
    //     todo!()
    // }

    // pub fn handle_signalling(&self, ...) {
    //     todo!()
    // }

    // pub fn handle_remote_inbound(&self, ...) { todo!() }
    // pub fn handle_remote_outbound(&self, ...) { todo!() }
}
