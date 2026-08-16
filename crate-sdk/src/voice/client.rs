use std::{net::SocketAddr, sync::Arc, time::Instant};

use common::{
    v1::types::voice::{
        VoiceState,
        datachannel::ProtocolType,
        messages::{SignallingCommand, SignallingEvent},
    },
    v2::types::ChannelId,
};
use futures_util::{StreamExt, stream::BoxStream};
use str0m::Rtc;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
    time,
};
use tracing::{error, info};

use crate::{
    Client,
    voice::{VoiceError, VoiceEvent},
};

pub(crate) struct VoiceInner {
    tx: mpsc::Sender<RtcCommand>,
    rx: broadcast::Sender<RtcEvent>,
}

/// a connection to a voice channel
pub struct Voice {
    state: Arc<VoiceInner>,
}

pub struct VoiceBuilder<'a> {
    client: &'a Client,
    channel_id: ChannelId,
    self_mute: bool,
    self_deaf: bool,
}

impl Voice {
    /// access the track registry
    pub fn tracks(&self) -> () {
        todo!()
    }
    // fn tracks_mut(&self) -> () {
    //     todo!()
    // }

    /// get the current voice state
    pub fn state(&self) -> &VoiceState {
        todo!()
    }

    /// get a stream of events
    pub fn events(&self) -> BoxStream<'static, VoiceEvent> {
        let rx = self.state.rx.subscribe();
        let rx = tokio_stream::wrappers::BroadcastStream::new(rx);
        rx.filter_map(|evt| async move {
            match evt {
                Ok(RtcEvent::Signalling(_cmd)) => {
                    // FIXME: Map RtcEvent to VoiceEvent
                    None
                }
                Err(_) => None,
            }
        })
        .boxed()
    }

    // /// get a stream of incoming tracks
    // pub fn inbound(&self) -> BoxStream<'static, Inbound> {
    //     futures_util::stream::empty().boxed()
    // }

    // /// create a new outgoing audio track
    // pub async fn create_audio<S: AudioSource>(
    //     &self,
    //     _source: S,
    // ) -> Result<OutboundPending, VoiceError> {
    //     todo!()
    // }

    // /// create a new outgoing video track
    // pub async fn create_video<S: VideoSource>(
    //     &self,
    //     _source: S,
    // ) -> Result<OutboundPending, VoiceError> {
    //     todo!()
    // }

    /// create a new datachannel
    pub async fn create_channel(&self, _protocol: ProtocolType) -> Result<(), VoiceError> {
        todo!()
    }

    // /// move to a different channel
    // ///
    // /// will attempt to recreate all existing tracks
    // pub async fn move_channel(&self, _channel_id: ChannelId) -> Result<(), VoiceError> {
    //     todo!()
    // }

    // pub async fn set_mute(&self, _mute: bool) -> Result<(), VoiceError> {
    //     todo!()
    // }

    // pub async fn set_deaf(&self, _deaf: bool) -> Result<(), VoiceError> {
    //     todo!()
    // }

    // pub async fn disconnect(self) -> Result<(), VoiceError> {
    //     todo!()
    // }
}

impl<'a> VoiceBuilder<'a> {
    pub fn new(client: &'a Client, channel_id: ChannelId) -> Self {
        VoiceBuilder {
            client,
            channel_id,
            self_mute: false,
            self_deaf: false,
        }
    }

    /// overwrite the channel id
    pub fn channel(mut self, channel_id: ChannelId) -> Self {
        self.channel_id = channel_id;
        self
    }

    /// set whether we're muted
    pub fn mute(mut self, mute: bool) -> Self {
        self.self_mute = mute;
        self
    }

    /// set whether we're deafened
    pub fn deaf(mut self, deaf: bool) -> Self {
        self.self_deaf = deaf;
        self
    }

    pub async fn connect(self) -> Result<Voice, VoiceError> {
        // TODO: return better error
        let _channel_id = self.channel_id;

        let rtc = Rtc::builder().build(Instant::now());

        // TODO: use stun to find public addr
        // TODO: don't panic
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        // // TODO: configurable stun addr
        // let stun_addr = "stun.l.google.com:19302"
        //     .to_socket_addrs()?
        //     .filter(|x| x.is_ipv4())
        //     .next()
        //     .unwrap();
        let sock = UdpSocket::bind(local_addr).await?;
        // let c = StunClient::new(stun_addr);
        // let f = c.query_external_address_async(&sock);
        // let addr = f.await.unwrap();
        // let candidate = Candidate::host(addr, "udp").unwrap();
        // debug!("listen on {}", sock.local_addr()?);
        // debug!("public addr {}", addr);
        // rtc.add_local_candidate(candidate);

        // TODO: Use self.client to send signaling packets and initialize WebRTC

        let (tx, rx) = mpsc::channel::<RtcCommand>(64);
        let (evt_tx, _) = broadcast::channel::<RtcEvent>(64);
        let worker = VoiceActor {
            rtc,
            rx,
            tx: evt_tx.clone(),
            sock,
            pending: None,
        };
        tokio::spawn(worker.spawn());

        let state = Arc::new(VoiceInner { tx, rx: evt_tx });
        Ok(Voice { state })
    }
}

// TODO: move below into new file?

/// sent to the worker
#[derive(Debug, Clone)]
pub enum RtcCommand {
    /// handle a signalling event from the server
    Signalling(SignallingEvent),
    // create/remove track
}

/// emitted by the worker
#[derive(Debug, Clone)]
pub enum RtcEvent {
    /// send this signalling command to the server
    Signalling(SignallingCommand),
}

pub struct VoiceActor {
    rtc: Rtc,
    rx: mpsc::Receiver<RtcCommand>,
    tx: broadcast::Sender<RtcEvent>,
    sock: UdpSocket,
    pending: Option<str0m::change::SdpPendingOffer>,
}

impl VoiceActor {
    pub async fn spawn(mut self) {
        loop {
            if let Err(e) = self.step().await {
                error!("rtc step error: {e}");
            }
        }
    }

    pub async fn step(&mut self) -> Result<(), VoiceError> {
        if !self.rtc.is_alive() {
            // TODO: handle rtc dead
            error!("rtc dead");
            return Err(VoiceError::Internal);
        }

        let output = match self.rtc.poll_output() {
            Ok(o) => o,
            Err(e) => {
                error!("rtc poll error: {e}");
                return Err(VoiceError::Rtc(e));
            }
        };

        let timeout = match output {
            str0m::Output::Timeout(instant) => instant,
            str0m::Output::Transmit(v) => {
                self.sock.send_to(&v.contents, v.destination).await?;
                return Ok(());
            }
            str0m::Output::Event(event) => {
                self.handle_str0m_event(event).await?;
                return Ok(());
            }
        };

        let mut packet_buf = vec![0; 2048];
        let sleep = time::sleep_until(time::Instant::from_std(timeout));

        tokio::select! {
            biased;

            Some(cmd) = self.rx.recv() => {
                self.handle_command(cmd).await ?;
                return Ok(())
            },

            Ok((n, source)) = self.sock.recv_from(&mut packet_buf) => {
                let res = self.rtc.handle_input(str0m::Input::Receive(
                    Instant::now(),
                    str0m::net::Receive {
                        proto: str0m::net::Protocol::Udp,
                        source,
                        destination: self.sock.local_addr()?,
                        contents: packet_buf[..n].try_into()?,
                    },
                ));
                if let Err(e) = res {
                    error!("rtc handle_input error: {e}");
                }
            }

            _ = sleep => {
                if let Err(e) = self.rtc.handle_input(str0m::Input::Timeout(Instant::now())) {
                    error!("rtc handle_input timeout error: {e}");
                    // TODO: what now?
                }
            },
        }

        Ok(())
    }

    pub async fn handle_command(&mut self, cmd: RtcCommand) -> Result<(), VoiceError> {
        match cmd {
            RtcCommand::Signalling(s) => match s {
                SignallingEvent::Connected { .. } => {
                    info!("Connected to SFU");
                }
                SignallingEvent::Disconnected => {
                    info!("Disconnected from SFU");
                }
                SignallingEvent::Offer { sdp, .. } => {
                    let sdp = str0m::change::SdpOffer::from_sdp_string(&sdp.0)?;
                    let answer = self.rtc.sdp_api().accept_offer(sdp)?;
                    self.tx
                        .send(RtcEvent::Signalling(SignallingCommand::Answer {
                            sdp: common::v1::types::voice::SessionDescription(
                                answer.to_sdp_string(),
                            ),
                        }))
                        .map_err(|_| VoiceError::Internal)?;
                }
                SignallingEvent::Answer { sdp } => {
                    let sdp = str0m::change::SdpAnswer::from_sdp_string(&sdp.0)?;
                    if let Some(pending) = self.pending.take() {
                        self.rtc.sdp_api().accept_answer(pending, sdp)?;
                    } else {
                        error!("got answer without a pending offer, ignoring");
                    }
                }
                SignallingEvent::Candidate { candidate } => {
                    if let Ok(c) = str0m::Candidate::from_sdp_string(&candidate.0) {
                        self.rtc.add_remote_candidate(c);
                    } else {
                        error!("failed to parse candidate: {}", candidate.0);
                    }
                }
                SignallingEvent::Tracks { .. } => {
                    todo!("handle tracks")
                }
                SignallingEvent::Subscribe(_subs) => {
                    todo!("handle subscribe")
                }
                SignallingEvent::Migrate { new_sfu_id } => {
                    info!("Migrating to SFU: {:?}", new_sfu_id);
                    todo!("handle migrate")
                }
                SignallingEvent::Error { message, code } => {
                    error!("Signalling error: {} ({:?})", message, code);
                }
            },
        }
        Ok(())
    }

    pub async fn handle_str0m_event(&mut self, event: str0m::Event) -> Result<(), VoiceError> {
        match event {
            str0m::Event::Connected => info!("player connected!"),
            _ => {}
        }

        Ok(())
    }
}
