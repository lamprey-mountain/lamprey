use lamprey_backend_core::config::ConfigVoice;
use std::{collections::HashMap, net::SocketAddr, time::Instant};

use bytes::{Bytes, BytesMut};
use common::v1::types::voice::messages::{SfuEvent, SignallingCommand};
use common::v1::types::voice::{Mid, Rid};
use common::v2::types::{ChannelId, UserId};
use slotmap::SlotMap;
use str0m::{Candidate, RtcConfig};
use tokio::{net::UdpSocket, sync::mpsc};
use tokio_stream::StreamExt;
use tokio_util::time::{DelayQueue, delay_queue::Key};
use tracing::{debug, warn};

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

    /// map channel id -> call slot
    channels: HashMap<ChannelId, CallSlot>,

    timeout_queue: DelayQueue<(CallSlot, PeerSlot)>,
    timeout_keys: HashMap<(CallSlot, PeerSlot), Key>,
}

#[derive(Clone)]
pub struct ShardHandle {
    control_tx: mpsc::Sender<ShardCommand>,
}

pub enum ShardCommand {
    /// create a new peer
    CreatePeer(SfuVoiceState),

    /// a signalling command that the user sent
    Signalling {
        channel_id: ChannelId,
        user_id: UserId,
        inner: SignallingCommand,
    },

    GenerateKeyframe {
        channel_id: ChannelId,
        user_id: UserId,
        mid: Mid,
        rid: Option<Rid>,
        kind: SKeyframeRequestKind,
    },
}

// enum ShardEvent {
//     // TODO
// }

impl Shard {
    pub async fn new(backend: BackendHandle, config: ConfigVoice) -> Result<(Self, ShardHandle)> {
        let (control_tx, control_rx) = mpsc::channel(100);

        let host_v4 = config
            .host_ipv4
            .as_deref()
            .ok_or_else(|| Error::Channel("host_ipv4 missing in config".into()))?;
        let host_v6 = config
            .host_ipv6
            .as_deref()
            .ok_or_else(|| Error::Channel("host_ipv6 missing in config".into()))?;

        let sock_v4 = UdpSocket::bind(format!("{host_v4}:0")).await?;
        let sock_v6 = UdpSocket::bind(format!("[{host_v6}]:0")).await?;

        let me = Self {
            backend,
            control_rx,
            sock_v4,
            sock_v6,
            calls: SlotMap::with_key(),
            addrs: HashMap::new(),
            ufrags: HashMap::new(),
            channels: HashMap::new(),
            timeout_queue: DelayQueue::new(),
            timeout_keys: HashMap::new(),
        };

        let handle = ShardHandle { control_tx };

        Ok((me, handle))
    }

    pub async fn run(mut self) {
        let mut buf_v4 = [0u8; 2000];
        let mut buf_v6 = [0u8; 2000];

        // TODO: clean up dead peers
        loop {
            // drain rtc output (transmits + events) for all calls
            self.drain_calls().await;

            // run sdp renegotiation and dispatch resulting offers/answers back to clients
            self.process_all_negotiations();

            tokio::select! {
                // TODO: warn if local_addr is None
                Ok((len, source)) = self.sock_v4.recv_from(&mut buf_v4) => {
                    if let Ok(dst) = self.sock_v4.local_addr() {
                        self.handle_udp(dst, source, Bytes::copy_from_slice(&buf_v4[..len]));
                    }
                }

                Ok((len, source)) = self.sock_v6.recv_from(&mut buf_v6) => {
                    if let Ok(dst) = self.sock_v6.local_addr() {
                        self.handle_udp(dst, source, Bytes::copy_from_slice(&buf_v6[..len]));
                    }
                }

                Some(cmd) = self.control_rx.recv() => {
                    self.handle_command(cmd);
                }

                Some(expired) = self.timeout_queue.next() => {
                    let (call_slot, peer_slot) = expired.into_inner();
                    self.timeout_keys.remove(&(call_slot, peer_slot));
                    if let Some(call) = self.calls.get_mut(call_slot) {
                        call.unpause(peer_slot);
                        call.handle_timeout(peer_slot);
                    }
                }
            }
        }
    }

    fn process_all_negotiations(&mut self) {
        for (call_slot, call) in self.calls.iter_mut() {
            // PERF: add fn channel_id() to ShardCall
            let channel_id = *self
                .channels
                .iter()
                .find_map(|(ch, &slot)| if slot == call_slot { Some(ch) } else { None })
                .unwrap();

            let events = call.process_sdp_negotiations();
            for (user_id, signalling_event) in events {
                if let Err(e) = self.backend.send(SfuEvent::VoiceDispatch {
                    user_id,
                    channel_id,
                    payload: Box::new(signalling_event),
                }) {
                    warn!("Failed to dispatch renegotiation event: {:?}", e);
                }
            }
        }
    }

    async fn drain_calls(&mut self) {
        for (call_slot, call) in self.calls.iter_mut() {
            let (transmits, timeouts) = call.drain();

            for (peer_slot, t) in timeouts {
                let key = self
                    .timeout_queue
                    .insert_at((call_slot, peer_slot), dbg!(t).into());
                self.timeout_keys.insert((call_slot, peer_slot), key);
            }

            for t in transmits {
                let res = if t.destination.is_ipv4() {
                    self.sock_v4.send_to(&t.contents, t.destination).await
                } else {
                    self.sock_v6.send_to(&t.contents, t.destination).await
                };
                if let Err(e) = res {
                    warn!("Failed to send UDP packet: {:?}", e);
                }
            }
        }
    }

    /// handle a udp packet from `dst` to `src` with data `data`
    fn handle_udp(&mut self, dst: SocketAddr, src: SocketAddr, data: Bytes) {
        let now = Instant::now();
        let input = SInput::Receive(
            now,
            str0m::net::Receive {
                proto: str0m::net::Protocol::Udp,
                source: src,
                destination: dst,
                contents: match data.as_ref().try_into() {
                    Ok(c) => c,
                    Err(_) => return,
                },
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

        let Some((call_id, peer_slot)) = peer_loc else {
            // TODO: warn/debug/trace log for unresolvable packets?
            return;
        };

        self.cancel_timeout(call_id, peer_slot);
        if let Some(call) = self.calls.get_mut(call_id) {
            call.handle_input(peer_slot, input);
        }
    }

    fn cancel_timeout(&mut self, call_slot: CallSlot, peer_slot: PeerSlot) {
        if let Some(key) = self.timeout_keys.remove(&(call_slot, peer_slot)) {
            self.timeout_queue.remove(&key);
        }
    }

    // fn handle_disconnect(&mut self, peer_id: PeerId) {
    // async fn handle_peer_event(&mut self, peer_id: PeerId, event: Event) {}

    /// handle a shard command
    fn handle_command(&mut self, cmd: ShardCommand) {
        match cmd {
            ShardCommand::CreatePeer(state) => {
                let channel_id = state.inner.channel_id;
                debug!(?channel_id, ?state.inner.user_id, "Shard: Creating peer");
                let call_slot = *self
                    .channels
                    .entry(channel_id)
                    .or_insert_with(|| self.calls.insert(ShardCall::new(channel_id)));
                let Some(call) = self.calls.get_mut(call_slot) else {
                    warn!(
                        ?channel_id,
                        "Shard: Failed to get call slot for create peer"
                    );
                    return;
                };

                let mut rtc = RtcConfig::new().set_ice_lite(true).build(Instant::now());

                if let Ok(addr) = self.sock_v4.local_addr() {
                    if let Ok(c) = Candidate::host(addr, "udp") {
                        rtc.add_local_candidate(c);
                    }
                }

                if let Ok(addr) = self.sock_v6.local_addr() {
                    if let Ok(c) = Candidate::host(addr, "udp") {
                        rtc.add_local_candidate(c);
                    }
                }

                let local_ufrag = rtc.direct_api().local_ice_credentials().ufrag.to_string();
                let peer_slot = call.create_peer(state, rtc);
                self.ufrags.insert(local_ufrag, (call_slot, peer_slot));
                debug!(?channel_id, "Shard: Peer created");
            }
            ShardCommand::Signalling {
                channel_id,
                user_id,
                inner,
            } => {
                debug!(
                    ?channel_id,
                    ?user_id,
                    ?inner,
                    "Shard: Handling signalling command"
                );
                let Some(&call_slot) = self.channels.get(&channel_id) else {
                    warn!(?channel_id, "Shard: Signalling for unknown channel");
                    return;
                };
                if let Some(call) = self.calls.get_mut(call_slot) {
                    let events = call.handle_signalling_by_user(user_id, inner);
                    for signalling_event in events {
                        if let Err(e) = self.backend.send(SfuEvent::VoiceDispatch {
                            user_id,
                            channel_id,
                            payload: Box::new(signalling_event),
                        }) {
                            warn!("Failed to dispatch signalling event: {:?}", e);
                        }
                    }
                }
            }
            ShardCommand::GenerateKeyframe {
                channel_id,
                user_id,
                mid,
                rid,
                kind,
            } => {
                debug!(?channel_id, ?user_id, ?mid, "Shard: Generating keyframe");
                let Some(&call_slot) = self.channels.get(&channel_id) else {
                    return;
                };
                if let Some(call) = self.calls.get_mut(call_slot) {
                    call.generate_keyframe(user_id, mid, rid, kind);
                }
            }
        }
    }
}

impl ShardHandle {
    // NOTE: maybe i should make this async?
    pub fn create_peer(&self, s: SfuVoiceState) {
        let _ = self.control_tx.try_send(ShardCommand::CreatePeer(s));
    }

    pub fn generate_keyframe(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        mid: Mid,
        rid: Option<Rid>,
        kind: SKeyframeRequestKind,
    ) {
        let _ = self.control_tx.try_send(ShardCommand::GenerateKeyframe {
            channel_id,
            user_id,
            mid,
            rid,
            kind,
        });
    }

    pub fn handle_signalling(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        inner: SignallingCommand,
    ) {
        let _ = self.control_tx.try_send(ShardCommand::Signalling {
            channel_id,
            user_id,
            inner,
        });
    }

    // pub fn handle_remote_inbound(&self, ...) { todo!() }
    // pub fn handle_remote_outbound(&self, ...) { todo!() }
}
