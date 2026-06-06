use std::{net::SocketAddr, sync::Arc, time::Instant};

use common::{
    v1::types::voice::{
        datachannel::DatachannelProtocol,
        messages::{SignallingCommand, SignallingEvent},
        VoiceState,
    },
    v2::types::ChannelId,
};
use futures_util::{stream::BoxStream, StreamExt};
use str0m::Rtc;
use tokio::{net::UdpSocket, sync::mpsc, time};
use tracing::{error, info};

use crate::{
    voice::{
        player::{AudioSource, VideoSource},
        track::{Inbound, OutboundPending},
        VoiceError, VoiceEvent,
    },
    Client,
};

pub(crate) struct ConnectionState {
    tx: mpsc::Sender<RtcCommand>,
}

/// a connection to a voice channel
pub struct Peer {
    state: Arc<ConnectionState>,
}

pub struct PeerBuilder<'a> {
    client: &'a Client,
    channel_id: ChannelId,
    self_mute: bool,
    self_deaf: bool,
}

impl Peer {
    /// get a stream of events
    pub fn events(&self) -> BoxStream<'static, VoiceEvent> {
        futures_util::stream::empty().boxed()
    }

    /// get a stream of incoming tracks
    pub fn inbound(&self) -> BoxStream<'static, Inbound> {
        futures_util::stream::empty().boxed()
    }

    /// create a new outgoing audio track
    pub async fn create_audio<S: AudioSource>(
        &self,
        _source: S,
    ) -> Result<OutboundPending, VoiceError> {
        todo!()
    }

    /// create a new outgoing video track
    pub async fn create_video<S: VideoSource>(
        &self,
        _source: S,
    ) -> Result<OutboundPending, VoiceError> {
        todo!()
    }

    /// create a new datachannel
    pub async fn create_channel(&self, _protocol: DatachannelProtocol) -> Result<(), VoiceError> {
        todo!()
    }
}

impl<'a> PeerBuilder<'a> {
    pub fn new(client: &'a Client, channel_id: ChannelId) -> Self {
        PeerBuilder {
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

    pub async fn connect(self) -> Result<Peer, VoiceError> {
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
        let worker = PeerWorker { rtc, rx, sock };
        tokio::spawn(worker.spawn());

        let state = Arc::new(ConnectionState { tx });
        Ok(Peer { state })
    }
}

// peer voice state methods
impl Peer {
    /// get the current voice state
    pub fn state(&self) -> &VoiceState {
        todo!()
    }

    /// move to a different channel
    ///
    /// will attempt to recreate all existing tracks
    pub async fn move_channel(&self, _channel_id: ChannelId) -> Result<(), VoiceError> {
        todo!()
    }

    pub async fn set_mute(&self, _mute: bool) -> Result<(), VoiceError> {
        todo!()
    }

    pub async fn set_deaf(&self, _deaf: bool) -> Result<(), VoiceError> {
        todo!()
    }

    pub async fn disconnect(self) -> Result<(), VoiceError> {
        todo!()
    }
}

/// sent to the worker
pub enum RtcCommand {
    /// handle a signalling event from the server
    Signalling(SignallingEvent),
    // create/remove track
}

/// emitted by the worker
pub enum RtcEvent {
    /// send this signalling command to the server
    Signalling(SignallingCommand),
}

pub struct PeerWorker {
    rtc: Rtc,
    rx: mpsc::Receiver<RtcCommand>,
    sock: UdpSocket,
    // pending: Option<SdpPendingOffer>,
    // tx: Sender<RtcEvent>,
}

impl PeerWorker {
    pub async fn spawn(mut self) {
        loop {
            if let Err(e) = self.step().await {
                error!("rtc step error: {e}");
            }
        }
    }

    pub async fn step(&mut self) -> Result<(), VoiceError> {
        if !self.rtc.is_alive() {
            todo!("handle rtc dead");
        }

        let output = match self.rtc.poll_output() {
            Ok(o) => o,
            Err(e) => {
                error!("rtc poll error: {e}");
                todo!("handle rtc poll error")
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
        // adding a track
        // let mut changes = self.rtc.sdp_api();
        // changes.add_media(
        //     str0m::media::MediaKind::Audio,
        //     str0m::media::Direction::SendOnly,
        //     None,
        //     None,
        //     None,
        // );
        // changes.apply();

        match cmd {
            RtcCommand::Signalling(s) => match s {
                SignallingEvent::Connected { .. } => todo!("info!"),
                SignallingEvent::Disconnected => todo!("disconnect"),
                SignallingEvent::Offer { sdp, tracks } => todo!(),
                SignallingEvent::Answer { sdp } => todo!(),
                SignallingEvent::Candidate { candidate } => todo!(),
                SignallingEvent::Tracks { user_id, tracks } => todo!(),
                SignallingEvent::Subscribe { subs } => todo!(),
                SignallingEvent::Migrate { new_sfu_id } => todo!(),
                SignallingEvent::Error { message, code } => todo!(),
            },
        }
    }

    pub async fn handle_str0m_event(&mut self, event: str0m::Event) -> Result<(), VoiceError> {
        match event {
            str0m::Event::Connected => info!("player connected!"),
            _ => {}
        }

        Ok(())
    }
}
