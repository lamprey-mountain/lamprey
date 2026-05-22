use common::v1::types::{
    voice::{Mid, Speaking, SpeakingFlags, SpeakingWithoutUserId, VoiceState, VoiceStateUpdate},
    ChannelId,
};

/// a connection to a voice channel
pub struct VoiceConnection {
    /// rtc instance for incoming media
    rtc_incoming: Rtc,

    /// rtc instance for outgoing media
    rtc_outgoing: Rtc,
    // TODO
    // sock: UdpSocket,
    // pending: Option<SdpPendingOffer>,
}

pub enum VoiceConnectionState {
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

pub struct VoiceTrackOutgoing<'voice> {
    voice: &'voice VoiceConnection,
    // TODO
}

pub struct VoiceTrackIncoming<'voice> {
    voice: &'voice VoiceConnection,
    // TODO
}

/// future that resolves when certain voice connection updates happen
///
/// specifically, disconnecting or moving from a channel
pub struct VoiceConnectionSwitcheroo;

impl VoiceConnectionSwitcheroo {
    fn state(&self) -> &VoiceState {
        todo!()
    }
}

impl std::future::Future for VoiceConnectionSwitcheroo {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

impl VoiceConnection {
    pub fn state(&self) -> &VoiceState {
        todo!()
    }

    pub fn disconnect(self) -> VoiceConnectionSwitcheroo {
        todo!()
    }

    pub fn move_channel(&self, channel_id: ChannelId) -> VoiceConnectionSwitcheroo {
        todo!()
    }

    pub fn send_speaking(&self, flags: SpeakingFlags, mid: Mid) {
        todo!()
    }
}

impl VoiceTrackOutgoing {
    pub fn mid(&self) -> Mid {
        todo!()
    }

    /// send media to this track
    pub fn send(&self) {
        todo!()
    }
}

impl VoiceTrackIncoming {
    pub fn mid(&self) -> Mid {
        todo!()
    }

    /// poll for media from this track
    pub async fn poll_media(&self) -> Option<()> {
        todo!()
    }
}

impl Client {
    /// connect to a voice channel
    pub fn voice(&self, state: VoiceStateUpdate) -> VoiceConnection {
        // let mut rtc = Rtc::new();

        // let local_addr: SocketAddr = "0.0.0.0:0".parse()?;
        // let stun_addr = "stun.l.google.com:19302"
        //     .to_socket_addrs()?
        //     .filter(|x| x.is_ipv4())
        //     .next()
        //     .unwrap();
        // let sock = UdpSocket::bind(local_addr).await?;
        // let c = StunClient::new(stun_addr);
        // let f = c.query_external_address_async(&sock);
        // let addr = f.await.unwrap();

        // let candidate = Candidate::host(addr, "udp")?;
        // debug!("listen on {}", sock.local_addr()?);
        // debug!("public addr {}", addr);
        // rtc.add_local_candidate(candidate);

        todo!()
    }
}
