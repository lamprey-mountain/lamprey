use std::{net::IpAddr, time::Instant};
use systemstat::{Platform, System};
use tokio::time;

use clap::Parser;
use common::v1::types::{
    voice::{
        MediaKindSerde, SessionDescription, SignallingMessage, TrackMetadata, VoiceState,
        VoiceStateUpdate,
    },
    Message, MessageClient, MessageCreate, MessageType, Session, User, UserId,
};
use figment::providers::{Env, Format, Toml};
use sdk::{Client, EventHandler, Http};
use str0m::{
    change::{SdpAnswer, SdpOffer, SdpPendingOffer},
    format::Codec,
    media::{Frequency, MediaTime, Mid},
    net::Protocol,
    Candidate, Rtc,
};
use symphonia::core::{
    codecs::CODEC_TYPE_OPUS,
    formats::{FormatOptions, FormatReader, Track},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use tokio::{
    net::UdpSocket,
    sync::mpsc::{Receiver, Sender},
};
use tracing::{debug, error, info, warn};

use crate::config::Config;

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(Toml::file("bot.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    tracing_subscriber::fmt()
        .with_env_filter(&config.rust_log)
        .init();

    info!("config {config:#?}");

    let mut client = Client::new(config.token.into());
    client.http = if let Some(base_url) = config.base_url {
        client.http.with_base_url(base_url.parse()?)
    } else {
        client.http
    };
    client.syncer = if let Some(ws_url) = config.ws_url {
        client.syncer.with_base_url(ws_url.parse()?)
    } else {
        client.syncer
    };

    let (control, control_events) = tokio::sync::mpsc::channel(100);
    let handle = Handle {
        http: client.http,
        voice_states: vec![],
        control,
        user: None,
        player: None,
    };
    client
        .syncer
        .with_handler(Box::new(handle))
        .with_controller(control_events)
        .connect()
        .await?;

    Ok(())
}

struct Handle {
    http: Http,
    voice_states: Vec<VoiceState>,
    control: Sender<MessageClient>,
    user: Option<User>,
    player: Option<Sender<PlayerCommand>>,
}

impl Handle {
    async fn send_signalling(&self, msg: SignallingMessage) -> anyhow::Result<()> {
        if let Some(user) = &self.user {
            self.control
                .send(MessageClient::VoiceDispatch {
                    user_id: user.id,
                    payload: serde_json::to_value(msg)?,
                })
                .await?;
        } else {
            warn!("no user for this connection");
        }
        Ok(())
    }
}

impl EventHandler for Handle {
    type Error = anyhow::Error;

    async fn ready(&mut self, user: Option<User>, _session: Session) -> Result<(), Self::Error> {
        if let Some(user) = user {
            info!("logged in as {}!", user.name);
            self.user = Some(user);
        } else {
            error!("no user for this token!");
            anyhow::bail!("no user for this token!");
        }

        Ok(())
    }

    async fn message_create(&mut self, message: Message) -> Result<(), Self::Error> {
        let content = match &message.message_type {
            MessageType::DefaultMarkdown(m) => m.content.as_deref(),
            // MessageType::MessagePinned(message_pin) => todo!(),
            // MessageType::MessageUnpinned(message_pin) => todo!(),
            // MessageType::MemberAdd(message_member) => todo!(),
            // MessageType::MemberRemove(message_member) => todo!(),
            // MessageType::MemberJoin(message_member) => todo!(),
            // MessageType::Call(message_call) => todo!(),
            // MessageType::ThreadRename(message_thread_rename) => todo!(),
            // MessageType::ThreadPingback(message_thread_pingback) => todo!(),
            _ => None,
        };

        if let Some(content) = content {
            debug!("message from {}: {}", message.author_id, content);
        } else {
            debug!("message from {} without content", message.author_id);
        }

        if let Some(command) = content.and_then(|c| c.strip_prefix("!")) {
            debug!("got raw command {command:?}");
            let command =
                Command::try_parse_from(std::iter::once("bot").chain(command.split_whitespace()))?;
            debug!("got command {command:?}");
            let resp = match command {
                Command::Ping => "pong!",
                Command::VoiceJoin => {
                    let author_voice_state = self
                        .voice_states
                        .iter()
                        .find(|s| s.user_id == message.author_id);
                    if let Some(author_voice_state) = author_voice_state {
                        self.send_signalling(SignallingMessage::VoiceState {
                            state: Some(VoiceStateUpdate {
                                thread_id: author_voice_state.thread_id,
                            }),
                        })
                        .await?;
                        "joined"
                    } else {
                        "you aren't in a voice thread"
                    }
                }
                Command::VoiceLeave => {
                    self.send_signalling(SignallingMessage::VoiceState { state: None })
                        .await?;
                    "left"
                }
                Command::VoicePlay => {
                    let (controller, events) = tokio::sync::mpsc::channel(100);
                    match Player::new(events).await {
                        Ok((player, offer)) => {
                            debug!("created player");
                            info!("sending offer: {}", offer.to_sdp_string());
                            self.send_signalling(SignallingMessage::Offer {
                                sdp: SessionDescription(offer.to_sdp_string().into()),
                                tracks: vec![TrackMetadata {
                                    mid: player.mid.to_string(),
                                    kind: MediaKindSerde::Audio,
                                    key: "music".into(),
                                }],
                            })
                            .await?;
                            tokio::spawn(player.run());
                            debug!("spawned player");
                            self.player = Some(controller);
                            "ok?"
                        }
                        Err(err) => {
                            error!("failed to create player: {err}");
                            "failed!"
                        }
                    }
                }
            };
            let resp = MessageCreate {
                content: Some(resp.into()),
                attachments: vec![],
                metadata: None,
                reply_id: Some(message.id),
                override_name: None,
                nonce: None,
                embeds: vec![],
                created_at: None,
            };
            self.http.message_create(message.thread_id, &resp).await?;
        }

        Ok(())
    }

    async fn voice_state(
        &mut self,
        user_id: UserId,
        state: Option<VoiceState>,
    ) -> Result<(), Self::Error> {
        debug!("got voice state for {user_id}: {state:?}");
        self.voice_states.retain(|s| s.user_id != user_id);
        if let Some(state) = state {
            self.voice_states.push(state);
        }
        Ok(())
    }

    async fn voice_dispatch(
        &mut self,
        user_id: UserId,
        payload: SignallingMessage,
    ) -> Result<(), Self::Error> {
        debug!("got voice dispatch for {user_id}: {payload:?}");
        match payload {
            SignallingMessage::Offer { .. } => {
                warn!("received offer, should impl renegotiation");
            }
            SignallingMessage::Answer { sdp } => {
                self.player
                    .as_mut()
                    .unwrap()
                    .send(PlayerCommand::Answer(
                        SdpAnswer::from_sdp_string(&sdp).unwrap(),
                    ))
                    .await?
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
enum Command {
    Ping,
    VoiceJoin,
    VoiceLeave,
    VoicePlay,
}

struct Player {
    rtc: Rtc,
    mid: Mid,
    pending: Option<SdpPendingOffer>,
    sock: UdpSocket,
    controller: Receiver<PlayerCommand>,
    format: Box<dyn FormatReader>,
    track: Track,
}

#[derive(Debug)]
enum PlayerCommand {
    Answer(SdpAnswer),
}

impl Player {
    pub async fn new(controller: Receiver<PlayerCommand>) -> anyhow::Result<(Self, SdpOffer)> {
        // init webrtc
        debug!("init webrtc");
        let mut rtc = Rtc::new();
        let mut changes = rtc.sdp_api();
        let mid = changes.add_media(
            str0m::media::MediaKind::Audio,
            str0m::media::Direction::SendOnly,
            None,
            None,
            None,
        );
        let (offer, pending) = changes.apply().unwrap();

        let addr = select_host_address_ipv4();
        let sock = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(sock.local_addr()?, "udp")?;
        debug!("listen on {}", sock.local_addr().unwrap());
        rtc.add_local_candidate(candidate);

        // init audio
        debug!("init audio");
        let file = std::fs::File::open("./music.opus")?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("opus");
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec == CODEC_TYPE_OPUS)
            .expect("no supported audio tracks")
            .clone();

        debug!("rtc ready");
        Ok((
            Self {
                controller,
                sock,
                rtc,
                mid,
                pending: Some(pending),
                format,
                track,
            },
            offer,
        ))
    }

    pub fn handle_command(&mut self, cmd: PlayerCommand) {
        debug!("handle command {cmd:?}");
        match cmd {
            PlayerCommand::Answer(answer) => {
                self.rtc
                    .sdp_api()
                    .accept_answer(self.pending.take().unwrap(), answer)
                    .unwrap();
            }
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("start run loop");
        let mut play_interval = time::interval(std::time::Duration::from_millis(20));
        play_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
        let mut media_ready = false;
        let track_id = self.track.id;

        loop {
            let timeout = match self.rtc.poll_output() {
                Ok(o) => o,
                Err(e) => {
                    error!("rtc poll error: {e}");
                    return Err(e.into());
                }
            };

            let timeout = match timeout {
                str0m::Output::Timeout(instant) => instant,
                str0m::Output::Transmit(v) => {
                    if let Err(e) = self.sock.send_to(&v.contents, v.destination).await {
                        error!("sock send error: {e}");
                    }
                    continue;
                }
                str0m::Output::Event(event) => {
                    debug!("{event:?}");
                    match event {
                        str0m::Event::Connected => {
                            info!("player connected!");
                            media_ready = true;
                        }
                        _ => {}
                    }
                    continue;
                }
            };

            let mut packet_buf = vec![0; 2048];
            let sleep = time::sleep_until(time::Instant::from_std(timeout));

            tokio::select! {
                biased;

                Some(cmd) = self.controller.recv() => self.handle_command(cmd),

                Ok((n, source)) = self.sock.recv_from(&mut packet_buf) => {
                    debug!("sock recv");
                    let res = self.rtc.handle_input(str0m::Input::Receive(
                        Instant::now(),
                        str0m::net::Receive {
                            proto: Protocol::Udp,
                            source,
                            destination: self.sock.local_addr()?,
                            contents: packet_buf[..n].try_into()?,
                        },
                    ));
                    if let Err(e) = res {
                        error!("rtc handle_input error: {e}");
                    }
                }

                _ = play_interval.tick(), if media_ready => {
                    match self.format.next_packet() {
                        Ok(packet) => {
                            if packet.track_id() != track_id {
                                continue;
                            }

                            let writer = if let Some(w) = self.rtc.writer(self.mid) {
                                w
                            } else {
                                warn!("writer for mid not available");
                                media_ready = false;
                                continue;
                            };

                            let pt = if let Some(p) = writer
                                .payload_params()
                                .find(|t| t.spec().codec == Codec::Opus)
                            {
                                p.pt()
                            } else {
                                warn!("opus codec not supported by peer");
                                media_ready = false;
                                continue;
                            };

                            let base = self.track.codec_params.time_base.unwrap();
                            let time = MediaTime::new(
                                packet.ts(),
                                Frequency::new(base.denom).unwrap(),
                            );
                            if let Err(e) = writer.write(pt, Instant::now(), time, packet.data) {
                                error!("failed to write rtp packet: {e}");
                            }
                        }
                        Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            info!("song finished");
                            media_ready = false; // Stop trying to play
                        }
                        Err(e) => {
                            error!("failed to read packet: {e}");
                            media_ready = false; // Stop trying to play
                        }
                    }
                }

                _ = sleep => {
                    if let Err(e) = self.rtc.handle_input(str0m::Input::Timeout(Instant::now())) {
                        error!("rtc handle_input timeout error: {e}");
                    }
                },
            }
        }
    }
}

pub fn select_host_address_ipv4() -> IpAddr {
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() && !v.is_private() {
                    debug!("selected ipv4 addr {v}");
                    return IpAddr::V4(v);
                }
            }
        }
    }

    panic!("Found no usable network interface");
}
