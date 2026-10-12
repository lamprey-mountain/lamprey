use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    time::Instant,
};

use common::v2::types::ChannelId;
use futures::future::BoxFuture;
use str0m::{Candidate, Rtc};
use stunclient::StunClient;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
};
use tracing::debug;

use crate::{
    Client,
    voice::{
        VoiceError,
        client::{Shared, Voice},
        driver::{Driver, RtcCommand},
    },
};

pub struct VoiceBuilder<'a> {
    client: &'a Client,
    channel_id: ChannelId,
    self_mute: bool,
    self_deaf: bool,
}

impl<'a> VoiceBuilder<'a> {
    pub(super) fn new(client: &'a Client, channel_id: ChannelId) -> Self {
        VoiceBuilder {
            client,
            channel_id,
            self_mute: false,
            self_deaf: false,
        }
    }

    /// change which channel to connect to
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

    async fn connect(self) -> Result<Voice, VoiceError> {
        let channel_id = self.channel_id;

        // find public addr via stun
        // TODO: don't panic
        // TODO: configurable stun addr
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let sock = UdpSocket::bind(local_addr).await?;
        let stun_addr = "stun.l.google.com:19302"
            .to_socket_addrs()?
            .filter(|x| x.is_ipv4()) // TODO: support ipv6
            .next()
            .unwrap();
        let c = StunClient::new(stun_addr);
        let addr = c.query_external_address_async(&sock).await.unwrap();
        debug!("listen on {}", sock.local_addr()?);
        debug!("public addr {}", addr);

        let candidate = Candidate::host(addr, "udp").unwrap();
        let mut rtc = Rtc::builder().build(Instant::now());
        rtc.add_local_candidate(candidate);

        let (tx, rx) = mpsc::channel::<RtcCommand>(64);
        // let (evt_tx, _) = broadcast::channel::<RtcEvent>(64);
        let worker = Driver::new(
            rtc,
            rx,
            self.client.syncer(),
            sock,
            channel_id,
        );
        tokio::spawn(worker.spawn());

        // let state = Arc::new(Shared { tx, rx: evt_tx });
        let state = Arc::new(Shared { tx });
        Ok(Voice { state })
    }
}

impl<'a> IntoFuture for VoiceBuilder<'a> {
    type Output = Result<Voice, VoiceError>;
    type IntoFuture = BoxFuture<'a, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}
