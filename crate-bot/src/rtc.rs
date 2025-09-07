use std::{net::IpAddr, path::PathBuf, time::Instant};
use systemstat::{Platform, System};
use tokio::time;

use anyhow::{anyhow, Result};
use common::v1::types::voice::{
    MediaKindSerde, SessionDescription, SignallingMessage, TrackMetadata,
};
use str0m::{
    change::{SdpAnswer, SdpPendingOffer},
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

pub struct Player {
    rtc: Rtc,
    mid: Mid,
    pending: Option<SdpPendingOffer>,
    sock: UdpSocket,
    controller: Receiver<PlayerCommand>,
    emitter: Sender<PlayerEvent>,
    audio: Option<PlayerAudio>,
}

struct PlayerAudio {
    format: Box<dyn FormatReader>,
    track: Track,
}

#[derive(Debug)]
pub enum PlayerCommand {
    Signalling(SignallingMessage),
    Play(PathBuf),
}

#[derive(Debug)]
pub enum PlayerEvent {
    Signalling(SignallingMessage),
}

impl Player {
    pub async fn new(
        controller: Receiver<PlayerCommand>,
        emitter: Sender<PlayerEvent>,
    ) -> anyhow::Result<Self> {
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

        emitter
            .send(PlayerEvent::Signalling(SignallingMessage::Offer {
                sdp: SessionDescription(offer.to_sdp_string().into()),
                tracks: vec![TrackMetadata {
                    mid: mid.to_string(),
                    kind: MediaKindSerde::Audio,
                    key: "music".into(),
                }],
            }))
            .await?;

        let addr = select_host_address_ipv4()?;
        let sock = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(sock.local_addr()?, "udp")?;
        debug!("listen on {}", sock.local_addr().unwrap());
        rtc.add_local_candidate(candidate);

        debug!("rtc ready");
        Ok(Self {
            controller,
            emitter,
            sock,
            rtc,
            mid,
            pending: Some(pending),
            audio: None,
        })
    }

    pub fn handle_command(&mut self, cmd: PlayerCommand) -> anyhow::Result<()> {
        debug!("handle command {cmd:?}");
        match cmd {
            PlayerCommand::Signalling(msg) => {
                match msg {
                    SignallingMessage::Offer { .. } => {
                        warn!("received offer, should impl renegotiation");
                    }
                    SignallingMessage::Answer { sdp } => {
                        let sdp = SdpAnswer::from_sdp_string(&sdp).unwrap();
                        self.rtc
                            .sdp_api()
                            .accept_answer(self.pending.take().unwrap(), sdp)
                            .unwrap();
                    }
                    _ => {} // TODO: handle other messages
                }
            }
            PlayerCommand::Play(path_buf) => {
                debug!("init audio");

                let mut hint = Hint::new();
                if let Some(ext) = path_buf.extension().and_then(|ext| ext.to_str()) {
                    hint.with_extension(ext);
                }

                let file = std::fs::File::open(path_buf)?;
                let mss = MediaSourceStream::new(Box::new(file), Default::default());
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

                self.audio = Some(PlayerAudio { track, format });
            }
        }
        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("start run loop");
        let mut play_interval = time::interval(std::time::Duration::from_millis(20));
        play_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
        let mut media_ready = false;

        loop {
            if !self.rtc.is_alive() {
                info!("rtc dead");
                // return Ok(());
                todo!("cleanup this!");
            }

            self.negotiate_if_needed()?;

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

                Some(cmd) = self.controller.recv() => {
                    if let Err(e) = self.handle_command(cmd) {
                        error!("handle_command error: {e}");
                    }
                },

                Ok((n, source)) = self.sock.recv_from(&mut packet_buf) => {
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
                    self.play_audio()
                }

                _ = sleep => {
                    if let Err(e) = self.rtc.handle_input(str0m::Input::Timeout(Instant::now())) {
                        error!("rtc handle_input timeout error: {e}");
                    }
                },
            }
        }
    }

    fn play_audio(&mut self) {
        let Some(audio) = &mut self.audio else {
            return;
        };

        let track_id = audio.track.id;
        let packet = match audio.format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                info!("song finished");
                self.audio = None;
                return;
            }
            Err(e) => {
                error!("failed to read packet: {e}");
                self.audio = None;
                return;
            }
        };

        if packet.track_id() != track_id {
            return;
        }

        let writer = if let Some(w) = self.rtc.writer(self.mid) {
            w
        } else {
            warn!("writer for mid not available");
            self.audio = None;
            return;
        };

        let pt = if let Some(p) = writer
            .payload_params()
            .find(|t| t.spec().codec == Codec::Opus)
        {
            p.pt()
        } else {
            warn!("opus codec not supported by peer");
            self.audio = None;
            return;
        };

        let base = audio.track.codec_params.time_base.unwrap();
        let time = MediaTime::new(packet.ts(), Frequency::new(base.denom).unwrap());
        if let Err(e) = writer.write(pt, Instant::now(), time, packet.data) {
            error!("failed to write rtp packet: {e}");
        }
    }

    fn negotiate_if_needed(&mut self) -> Result<bool> {
        todo!()

        // if matches!(self.signalling_state, SignallingState::HaveLocalOffer(_)) {
        //     // NOTE: do i overwrite the pending offer here?
        //     warn!("trying to negotiate, but we already have a local offer");
        //     return Ok(false);
        // }

        // let mut change = self.rtc.sdp_api();

        // for track in &mut self.outbound {
        //     if track.state == TrackState::Pending {
        //         let mid = change.add_media(
        //             track.kind,
        //             Direction::SendOnly,
        //             // Some(track.ssrc.clone()),
        //             None,
        //             None,
        //             None,
        //         );
        //         track.state = TrackState::Negotiating(mid);
        //     }
        // }

        // if !change.has_changes() {
        //     return Ok(false);
        // }

        // let Some((offer, pending)) = change.apply() else {
        //     return Ok(false);
        // };

        // self.emit(PeerEvent::Signalling(SignallingMessage::Offer {
        //     sdp: SessionDescription(offer.to_sdp_string()),
        //     tracks: vec![],
        // }))?;
        // self.signalling_state = SignallingState::HaveLocalOffer(pending);

        // Ok(true)
    }
}

fn select_host_address_ipv4() -> Result<IpAddr> {
    let system = System::new();
    let networks = system.networks().unwrap();

    for net in networks.values() {
        for n in &net.addrs {
            if let systemstat::IpAddr::V4(v) = n.addr {
                if !v.is_loopback() && !v.is_link_local() && !v.is_broadcast() && !v.is_private() {
                    debug!("selected ipv4 addr {v}");
                    return Ok(IpAddr::V4(v));
                }
            }
        }
    }

    Err(anyhow!("Found no usable network interface"))
}
