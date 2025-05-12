use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json,
};
use common::v1::types::{util::Time, voice::VoiceState, UserId};
use dashmap::DashMap;
use sfu::{Request, RtcPeerCommand, RtcPeerEvent};
use str0m::{
    change::{SdpAnswer, SdpOffer, SdpPendingOffer},
    format::PayloadParams,
    media::{MediaKind, MediaTime, Mid},
    net::{Protocol, Receive},
    Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig,
};
use tokio::{
    net::UdpSocket,
    select,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::sleep_until,
};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod util;

#[derive(Debug, Default)]
pub struct Sfu {
    peers: DashMap<UserId, UnboundedSender<PeerCommandPayload>>,
    voice_states: DashMap<UserId, VoiceState>,
}

#[derive(Debug)]
pub struct Peer {
    rtc: Rtc,
    socket: UdpSocket,
    packet: [u8; 2000],
    outbound: HashMap<Uuid, Mid>,
    inbound: HashMap<Mid, Uuid>,
    sdp_pending: Option<SdpPendingOffer>,
}

impl Peer {
    async fn spawn(
        sfu_send: UnboundedSender<PeerEventEnvelope>,
        user_id: UserId,
    ) -> Result<UnboundedSender<PeerCommandPayload>> {
        info!("create new peer {user_id}");

        let mut rtc = RtcConfig::new()
            .set_ice_lite(true)
            .set_stats_interval(Some(Duration::from_secs(5)))
            .build();

        let addr = util::select_host_address_ipv4();
        let socket = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(socket.local_addr()?, "udp")?;
        rtc.add_local_candidate(candidate);

        let peer = Self {
            rtc,
            socket,
            packet: [0; 2000],
            outbound: HashMap::new(),
            inbound: HashMap::new(),
            sdp_pending: None,
            // user_id: Some(user_id),
        };

        let (send, recv) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            if let Err(err) = peer.run(user_id, recv, sfu_send).await {
                error!("while running peer: {err:?}");
            }
        });

        Ok(send)
    }

    #[tracing::instrument(skip(self, user_id, a, events))]
    async fn run(
        mut self,
        user_id: UserId,
        mut a: UnboundedReceiver<PeerCommandPayload>,
        mut events: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        loop {
            let timeout = match self.rtc.poll_output()? {
                Output::Timeout(v) => v,
                Output::Transmit(v) => {
                    trace!("transmit {} bytes to {}", v.contents.len(), v.destination);
                    self.socket.send_to(&v.contents, v.destination).await?;
                    continue;
                }
                Output::Event(v) => {
                    trace!("{v:?}");
                    match v {
                        Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                            self.rtc.is_alive(); // TODO: disconnect and clean up if this is false
                                                 // self.rtc.disconnect();
                            return Result::Ok(());
                        }

                        Event::Connected => debug!("connected!"),

                        Event::MediaAdded(m) => {
                            debug!("media added {m:?}");
                            let id = Uuid::now_v7();
                            self.inbound.insert(m.mid, id);
                            events.send(PeerEventEnvelope {
                                user_id,
                                payload: PeerEventPayload::MediaAdded { kind: m.kind, id },
                            })?;
                        }

                        Event::MediaData(m) => {
                            if let Some(&id) = self.inbound.get(&m.mid) {
                                events.send(PeerEventEnvelope {
                                    user_id,
                                    payload: PeerEventPayload::MediaData {
                                        id,
                                        network_time: m.network_time,
                                        time: m.time,
                                        data: m.data,
                                        params: m.params,
                                    },
                                })?;
                            } else {
                                error!("recv data with no inbound id?")
                            };
                        }

                        Event::PeerStats(_)
                        | Event::MediaIngressStats(_)
                        | Event::MediaEgressStats(_)
                        | Event::EgressBitrateEstimate(_) => {
                            debug!("{v:?}");
                        }

                        _ => {}
                    };
                    continue;
                }
            };

            let input = select! {
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
                recv = a.recv() => {
                    self.handle(user_id, recv.expect("channel unexpectedly closed!"), &mut events).await?;
                    continue;
                },
            };

            // TODO: disconnect if this returns Err
            self.rtc.handle_input(input)?;
        }
    }

    async fn handle(
        &mut self,
        user_id: UserId,
        command: PeerCommandPayload,
        events: &mut UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        match command {
            PeerCommandPayload::RtcPeerCommand(cmd) => {
                debug!("handle peer command {cmd:?}");
                self.handle_command(user_id, cmd, events).await?;
            }
            PeerCommandPayload::MediaAdded { id, kind } => {
                debug!("handle peer command {command:?}");
                let mut changes = self.rtc.sdp_api();
                let mid =
                    changes.add_media(kind, str0m::media::Direction::SendOnly, None, None, None);
                debug!("create sendonly media mid = {mid}");

                if let Some((offer, pending)) = changes.apply() {
                    debug!("sdp offer {offer}");
                    self.sdp_pending = Some(pending);
                    send_rpc(&RpcCommand::VoiceDispatch {
                        user_id,
                        payload: RtcPeerEvent::Offer {
                            sdp: offer.to_sdp_string(),
                        },
                    })
                    .await?;
                } else {
                    debug!("don't need to send any pending message?");
                };
                self.outbound.insert(id, mid);
            }
            PeerCommandPayload::MediaData {
                id,
                network_time,
                time,
                data,
                params,
            } => {
                if let Some(mid) = self.outbound.get(&id) {
                    if let Some(writer) = self.rtc.writer(*mid) {
                        if let Some(pt) = writer.match_params(params) {
                            trace!("write {} bytes to {mid}", data.len());
                            if let Err(err) = writer.write(pt, network_time, time, data) {
                                warn!("client ({}) failed: {:?}", user_id, err);
                                self.rtc.disconnect();
                            }
                        } else {
                            error!("no matching pt?")
                        }
                    } else {
                        error!("recv data with no outbound writer?")
                    }
                } else {
                    error!("recv data with no outbound id?")
                };
            }
        }

        Ok(())
    }

    async fn handle_command(
        &mut self,
        user_id: UserId,
        command: RtcPeerCommand,
        sender: &mut UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        match command {
            RtcPeerCommand::Answer { sdp } => {
                let answer = SdpAnswer::from_sdp_string(&sdp)?;
                if let Some(pending) = self.sdp_pending.take() {
                    self.rtc.sdp_api().accept_answer(pending, answer)?;
                } else {
                    error!("got duplicate answer");
                }
            }
            RtcPeerCommand::Offer { sdp } => {
                let offer = SdpOffer::from_sdp_string(&sdp)?;
                let answer = self.rtc.sdp_api().accept_offer(offer)?;
                sender.send(PeerEventEnvelope {
                    user_id,
                    payload: PeerEventPayload::RtcPeerEvent(RtcPeerEvent::Answer {
                        sdp: answer.to_sdp_string(),
                    }),
                })?;
            }
            // RtcPeerCommand::IceCandidate { data } => {
            //     _ = dbg!(serde_json::from_str::<Candidate>(&data));
            // }
            _ => {}
        }
        Ok(())
    }
}

struct PeerEventEnvelope {
    user_id: UserId,
    payload: PeerEventPayload,
}

#[derive(Debug)]
enum PeerEventPayload {
    RtcPeerEvent(RtcPeerEvent),
    MediaAdded {
        id: Uuid,
        kind: MediaKind,
    },
    MediaData {
        id: Uuid,
        network_time: Instant,
        time: MediaTime,
        // TODO: use Arc<[u8]>
        data: Vec<u8>,
        params: PayloadParams,
    },
}

#[derive(Debug)]
enum PeerCommandPayload {
    RtcPeerCommand(RtcPeerCommand),
    MediaAdded {
        id: Uuid,
        kind: MediaKind,
    },
    MediaData {
        id: Uuid,
        network_time: Instant,
        time: MediaTime,
        // TODO: use Arc<[u8]>
        data: Vec<u8>,
        params: PayloadParams,
    },
}

type SfuCommand = Request;

impl Sfu {
    pub fn spawn(self) -> UnboundedSender<SfuCommand> {
        let (send, recv) = mpsc::unbounded_channel();
        tokio::spawn(self.asdf(recv));
        send
    }

    async fn asdf(self, mut a: UnboundedReceiver<SfuCommand>) -> Result<()> {
        let (peer_send, mut peer_events) = tokio::sync::mpsc::unbounded_channel();
        loop {
            tokio::select! {
                Some(req) = a.recv() => self.handle_command(req, peer_send.clone()).await?,
                Some(envelope) = peer_events.recv() => self.handle_event(envelope).await?,
            }
        }
    }

    async fn handle_command(
        &self,
        req: Request,
        peer_send: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        trace!("new rpc message {req:?}");

        let user_id = req.user_id.unwrap();
        let ctl = match self.peers.entry(user_id) {
            dashmap::Entry::Occupied(occupied_entry) => occupied_entry.into_ref(),
            dashmap::Entry::Vacant(vacant_entry) => {
                let peer = Peer::spawn(peer_send.clone(), user_id).await?;
                vacant_entry.insert(peer)
            }
        };

        match &req.inner {
            RtcPeerCommand::VoiceStateUpdate { patch } => {
                if let Some(thread_id) = patch.thread_id {
                    self.voice_states.insert(
                        user_id,
                        VoiceState {
                            user_id,
                            thread_id,
                            joined_at: Time::now_utc(),
                        },
                    );
                } else {
                    self.voice_states.remove(&user_id);
                }
                send_rpc(&RpcCommand::VoiceDispatch {
                    user_id,
                    payload: RtcPeerEvent::VoiceState {
                        user_id,
                        state: self.voice_states.get(&user_id).map(|s| s.to_owned()),
                    },
                })
                .await?;
                debug!("got voice state update {patch:?}");
            }
            _ => {}
        }

        ctl.send(PeerCommandPayload::RtcPeerCommand(req.inner))?;

        Ok(())
    }

    async fn handle_event(&self, envelope: PeerEventEnvelope) -> Result<()> {
        let user_id = envelope.user_id;
        let event = envelope.payload;
        match event {
            PeerEventPayload::RtcPeerEvent(RtcPeerEvent::Answer { sdp }) => {
                debug!("sdp answer {sdp}");
                send_rpc(&RpcCommand::VoiceDispatch {
                    user_id,
                    payload: RtcPeerEvent::Answer { sdp },
                })
                .await?;
            }

            PeerEventPayload::MediaAdded { kind, id } => {
                debug!("peer event payload {event:?}");
                for a in &self.peers {
                    if a.key() != &user_id {
                        a.value()
                            .send(PeerCommandPayload::MediaAdded { id, kind })?;
                    }
                }
            }
            PeerEventPayload::MediaData {
                id,
                network_time,
                time,
                data,
                params,
            } => {
                for a in &self.peers {
                    if a.key() != &user_id {
                        a.value().send(PeerCommandPayload::MediaData {
                            id,
                            network_time,
                            time,
                            data: data.clone(),
                            params,
                        })?;
                    }
                }
            }
            _ => todo!(),
        }

        Ok(())
    }
}

async fn start_http(wheel: UnboundedSender<SfuCommand>) -> Result<()> {
    let router = axum::Router::new()
        .route(
            "/rpc",
            post(|Json(req): Json<Request>| async move {
                // handles events proxied through the websocket
                if let Err(err) = wheel.send(req) {
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
    VoiceDispatch {
        user_id: UserId,
        payload: RtcPeerEvent,
    },
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

    let wheel = Sfu::default().spawn();
    let _ = start_http(wheel).await;

    Ok(())
}
