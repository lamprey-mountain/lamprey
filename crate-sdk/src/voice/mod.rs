use std::{
    future::{Future, IntoFuture},
    sync::Arc,
};

use bytes::Bytes;
use common::v1::types::{
    voice::{MediaKind, Mid, SpeakingFlags, SpeakingWithPeerId, VoiceState},
    ChannelId,
};
use futures_util::stream::{BoxStream, StreamExt};
use str0m::Rtc;
use tokio::sync::Mutex;

use crate::Client;

mod player;

struct ConnectionState {
    /// rtc instance for incoming media
    rtc_incoming: Mutex<Rtc>,

    /// rtc instance for outgoing media
    rtc_outgoing: Mutex<Rtc>,
}

/// a connection to a voice channel
pub struct VoiceConnection {
    state: Arc<ConnectionState>,
}

pub struct VoiceTrackOutgoing {
    state: Arc<ConnectionState>,
}

pub struct VoiceTrackOutgoingPending {
    state: Arc<ConnectionState>,
}

pub struct VoiceTrackIncoming {
    state: Arc<ConnectionState>,
}

pub enum VoiceEvent {
    /// voice connection state changed
    StateChanged(VoiceConnectionStatus),

    /// a user is speaking
    UserSpeaking(SpeakingWithPeerId),

    /// a new track was received
    Track(VoiceTrackIncoming),

    // ...
    Disconnected,
    Error(VoiceError),
}

pub enum VoiceError {
    Other,
}

pub enum VoiceConnectionStatus {
    /// disconnected
    Disconnected,

    /// sent a VoiceState update, waiting for an sfu to connect to
    AwaitingSfu,

    /// connecting to the sfu
    Connecting,

    /// connected to the sfu
    Connected,

    /// no route to host
    NoRoute,

    /// webrtc ice checking
    IceChecking,
}

/// currently connecting
pub struct VoiceConnectionPending;

impl VoiceConnectionPending {
    pub fn state(&self) -> &VoiceState {
        todo!()
    }
}

impl Future for VoiceConnectionPending {
    type Output = VoiceConnection;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

impl IntoFuture for VoiceConnectionBuilder<'_> {
    type Output = VoiceConnection;
    type IntoFuture = VoiceConnectionPending;

    fn into_future(self) -> Self::IntoFuture {
        todo!()
    }
}

impl VoiceConnection {
    /// get a stream of events
    pub fn events(&self) -> BoxStream<'static, VoiceEvent> {
        futures_util::stream::empty().boxed()
    }

    pub fn state(&self) -> &VoiceState {
        todo!()
    }

    pub async fn disconnect(self) -> Result<(), VoiceError> {
        todo!()
    }

    pub async fn move_channel(&self, _channel_id: ChannelId) -> Result<(), VoiceError> {
        todo!()
    }

    pub fn send_speaking(&self, _flags: SpeakingFlags, _mid: Mid) -> Result<(), VoiceError> {
        todo!()
    }

    /// create a new outgoing track
    pub async fn create_track(&self, _kind: MediaKind) -> Result<VoiceTrackOutgoing, VoiceError> {
        todo!()
    }

    pub async fn set_mute(&self, _mute: bool) -> Result<(), VoiceError> {
        todo!()
    }

    pub async fn set_deaf(&self, _deaf: bool) -> Result<(), VoiceError> {
        todo!()
    }
}

impl VoiceTrackOutgoing {
    pub fn mid(&self) -> Mid {
        todo!()
    }

    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    /// send media to this track
    pub async fn send(&self, _packet: MediaPacket) -> Result<(), VoiceError> {
        todo!()
    }
}

impl VoiceTrackOutgoingPending {
    pub fn kind(&self) -> MediaKind {
        todo!()
    }
}

impl Future for VoiceTrackOutgoingPending {
    type Output = VoiceTrackOutgoing;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

pub struct MediaPacket {
    data: Bytes,
}

impl VoiceTrackIncoming {
    pub fn mid(&self) -> Mid {
        todo!()
    }

    /// stream media from this track
    pub fn media_stream(&self) -> BoxStream<'static, MediaPacket> {
        futures_util::stream::empty().boxed()
    }
}

pub struct VoiceConnectionBuilder<'a> {
    client: &'a Client,
    channel_id: Option<ChannelId>,
    self_mute: bool,
    self_deaf: bool,
}

impl<'a> VoiceConnectionBuilder<'a> {
    pub fn channel(mut self, channel_id: ChannelId) -> Self {
        self.channel_id = Some(channel_id);
        self
    }

    pub fn mute(mut self, mute: bool) -> Self {
        self.self_mute = mute;
        self
    }

    pub fn deaf(mut self, deaf: bool) -> Self {
        self.self_deaf = deaf;
        self
    }

    pub async fn connect(self) -> Result<VoiceConnection, VoiceError> {
        let _channel_id = self.channel_id.ok_or(VoiceError::Other)?; // or a missing channel error

        // Use self.client to send signaling packets and initialize WebRTC

        todo!()
    }
}

impl Client {
    /// start configuring a voice connection
    pub fn voice(&self) -> VoiceConnectionBuilder<'_> {
        VoiceConnectionBuilder {
            client: self,
            channel_id: None,
            self_mute: false,
            self_deaf: false,
        }
    }

    // /// connect to a voice channel
    // pub fn voice(&self, state: VoiceStateUpdate) -> VoiceConnection {
    //     // let mut rtc = Rtc::new();

    //     // TODO: run this in background (return VoiceConnection immediately)
    //     // let local_addr: SocketAddr = "0.0.0.0:0".parse()?;
    //     // let stun_addr = "stun.l.google.com:19302"
    //     //     .to_socket_addrs()?
    //     //     .filter(|x| x.is_ipv4())
    //     //     .next()
    //     //     .unwrap();
    //     // let sock = UdpSocket::bind(local_addr).await?;
    //     // let c = StunClient::new(stun_addr);
    //     // let f = c.query_external_address_async(&sock);
    //     // let addr = f.await.unwrap();
    //     //
    //     // let candidate = Candidate::host(addr, "udp")?;
    //     // debug!("listen on {}", sock.local_addr()?);
    //     // debug!("public addr {}", addr);
    //     // rtc.add_local_candidate(candidate);

    //     todo!()
    // }
}
