use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json,
};
use common::v1::types::{util::Diff, RtcPeerId, UserId};
use dashmap::DashMap;
use serde_json::Value;
use sfu::{Request, RtcPeerCommand, RtcPeerEvent};
use str0m::{
    change::SdpOffer,
    media::{Direction, MediaData},
    net::{Protocol, Receive},
    Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig,
};
use systemstat::{Platform, System};
use tokio::{
    net::UdpSocket,
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::sleep_until,
};
use tracing::{debug, error, info, instrument::WithSubscriber, trace};
use tracing_subscriber::EnvFilter;

/// a flywheel
#[derive(Debug, Default)]
pub struct Wheel {
    peers: DashMap<UserId, RtcPeerController>,
    // calls: HashMap<ThreadId, Call>,
    // participants: HashMap<UserId, Participant>,
    // peers: HashMap<UserId, Peer>,

    // do this instead?
    // calls: Vec<Call>,

    // rest/admin api
    // PUT /config
    // POST /rpc -- ??? why have a rest api if im just gonna do this ???
    // GET /metrics

    // keep track of inbound, outbound bandwidth and latency. modify the topology as needed
}

#[derive(Debug, Default)]
pub struct Call {
    peers: Vec<RtcPeer>,
    // media: Vec<Media>,
}

// rename to Peer?
#[derive(Debug)]
pub struct RtcPeer {
    id: RtcPeerId,
    rtc: Rtc,
    socket: UdpSocket,
    packet: [u8; 2000],

    sender: UnboundedSender<RtcPeerEvent>,
    receiver: UnboundedReceiver<RtcPeerCommand>,

    // this is optional in case i want to retrofit in sfu to sfu federation. i'll make it required if that doesnt go anywhere.
    user_id: Option<UserId>,
    // have_media: Vec<Media>,
    wheel: Arc<Wheel>,
}

#[derive(Debug)]
pub struct RtcPeerController {
    pub send: UnboundedSender<RtcPeerCommand>,
    // pub recv: UnboundedReceiver<RtcPeerEvent>,
}

impl RtcPeer {
    pub async fn spawn(wheel: Arc<Wheel>, user_id: UserId) -> Result<RtcPeerController> {
        info!("create new peer {user_id}");

        let (parent_send, peer_recv) = unbounded_channel();
        let (peer_send, mut parent_recv) = unbounded_channel();

        let mut rtc = RtcConfig::new().set_ice_lite(true).build();

        let addr = select_host_address();
        let socket = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(socket.local_addr()?, "udp")?;
        rtc.add_local_candidate(candidate);

        let mut peer = Self {
            id: RtcPeerId::new(),
            rtc,
            socket,
            packet: [0; 2000],
            sender: peer_send,
            receiver: peer_recv,
            user_id: Some(user_id),
            wheel,
        };

        tokio::spawn(async move {
            if let Err(err) = peer.start().await {
                error!("while running peer: {err:?}");
            }
        });

        tokio::spawn(async move {
            while let Some(event) = parent_recv.recv().await {
                debug!("{event:?}");
                match event {
                    RtcPeerEvent::Answer { sdp } => {
                        debug!("sdp {sdp}");
                        send_rpc(&RpcCommand::VoiceDispatch {
                            user_id,
                            payload: serde_json::to_value(RtcPeerEvent::Answer { sdp }).unwrap(),
                        })
                        .await
                        .unwrap();
                    }
                }
            }
        });

        Ok(RtcPeerController {
            send: parent_send,
            // recv: parent_recv,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        loop {
            let timeout = match self.rtc.poll_output()? {
                Output::Timeout(v) => v,
                Output::Transmit(v) => {
                    debug!("transmit {} bytes to {}", v.contents.len(), v.destination);
                    self.socket.send_to(&v.contents, v.destination).await?;
                    continue;
                }
                Output::Event(v) => {
                    debug!("{v:?}");
                    match v {
                        Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                            // self.rtc.disconnect();
                            return Result::Ok(());
                        }

                        Event::Connected => debug!("connected!"),

                        // Event::MediaAdded(m) => {
                        //     for a in &self.wheel.peers {
                        //         a.value().send(RtcPeerCommand::MediaAdded(m));
                        //     }
                        // }

                        // Event::MediaData(m) => {
                        //     for a in &self.wheel.peers {
                        //         a.value().send(RtcPeerCommand::MediaData(m));
                        //     }
                        // }

                        // Event::ChannelOpen(channel_id, _) => todo!(),
                        // Event::ChannelData(channel_data) => todo!(),
                        // Event::ChannelClose(channel_id) => todo!(),
                        // Event::PeerStats(peer_stats) => todo!(),
                        // Event::MediaIngressStats(media_ingress_stats) => todo!(),
                        // Event::MediaEgressStats(media_egress_stats) => todo!(),
                        // Event::EgressBitrateEstimate(bwe_kind) => todo!(),
                        // Event::KeyframeRequest(keyframe_request) => todo!(),
                        // Event::StreamPaused(stream_paused) => todo!(),
                        _ => {}
                    };
                    continue;
                }
            };

            let input = loop {
                break select! {
                    _ = sleep_until(timeout.into()) => {
                        Input::Timeout(Instant::now())
                    }
                    recv = self.socket.recv_from(&mut self.packet) => {
                        let (n, source) = recv?;
                        Input::Receive(
                            Instant::now(),
                            Receive {
                                proto: Protocol::Udp,
                                source,
                                destination: self.socket.local_addr()?,
                                contents: self.packet[..n].try_into()?,
                            },
                        )
                    }
                    recv = self.receiver.recv() => {
                        self.handle_command(recv.expect("channel unexpectedly closed!")).await?;
                        continue;
                    }
                };
            };

            self.rtc.handle_input(input)?;
        }
    }

    pub async fn handle_command(&mut self, command: RtcPeerCommand) -> Result<()> {
        match command {
            RtcPeerCommand::Offer { sdp } => {
                let offer = SdpOffer::from_sdp_string(&sdp)?;
                let answer = self.rtc.sdp_api().accept_offer(offer)?;
                self.sender.send(RtcPeerEvent::Answer {
                    sdp: answer.to_sdp_string(),
                })?;
            } // RtcPeerCommand::MediaAdded(m) => {
              //     self.rtc
              //         .sdp_api()
              //         .add_media(m.kind, m.direction, None, None, None);
              // }
              // RtcPeerCommand::MediaData(m) => {
              //     self.rtc
              //         .writer(m.mid)?
              //         .write(m.pt, m.network_time, m.time, m.data)?;
              // }
        }
        Ok(())
    }
}

impl Wheel {
    // todo
}

pub async fn start_http(wheel: Arc<Wheel>) -> Result<()> {
    let s = wheel.clone();
    let router = axum::Router::new()
        .route(
            "/rpc",
            post(|Json(req): Json<Request>| async move {
                trace!("new rpc message {req:?}");

                let user_id = req.user_id.unwrap();
                // handles events proxied through the websocket?
                let ctl = match s.peers.entry(user_id) {
                    dashmap::Entry::Occupied(occupied_entry) => occupied_entry.into_ref(),
                    dashmap::Entry::Vacant(vacant_entry) => {
                        let peer = RtcPeer::spawn(s.clone(), user_id).await.unwrap();
                        vacant_entry.insert(peer)
                    }
                };

                if let Err(err) = ctl.send.send(req.inner) {
                    tracing::error!("{err}");
                };
                StatusCode::ACCEPTED
            }),
        )
        .route("/ping", get(|| async { StatusCode::NO_CONTENT }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type")]
enum RpcCommand {
    VoiceDispatch { user_id: UserId, payload: Value },
}

async fn send_rpc(command: &RpcCommand) -> Result<()> {
    reqwest::Client::new()
        .post("http://localhost:4000/api/v1/internal/rpc")
        .header("authorization", "Server verysecrettoken")
        .json(command)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    let wheel = Arc::new(Wheel::default());
    let _ = tokio::spawn(start_http(wheel)).await;

    Ok(())
}

pub fn select_host_address() -> IpAddr {
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() {
                    return IpAddr::V4(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}

pub fn select_host_address_v6() -> IpAddr {
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V6(v) = n.addr {
                if !v.is_loopback() && !v.is_unicast_link_local() && !v.is_multicast() {
                    return IpAddr::V6(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}
