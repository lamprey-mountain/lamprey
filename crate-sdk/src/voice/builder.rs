use std::{net::SocketAddr, sync::Arc, time::Instant};

use common::{
    v1::types::{MessageClient, voice::VoiceStateUpdate},
    v2::types::ChannelId,
};
use futures::future::BoxFuture;
use str0m::Rtc;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, watch},
};

use crate::{
    Client,
    voice::{
        ConnectionState, VoiceError,
        client::{LocalVoiceState, Shared, Voice},
        driver::{Driver, RtcCommand},
    },
};

/// a builder to connect to a voice channel
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
    #[inline]
    pub fn channel(mut self, channel_id: ChannelId) -> Self {
        self.channel_id = channel_id;
        self
    }

    /// set whether we're muted
    #[inline]
    pub fn mute(mut self, mute: bool) -> Self {
        self.self_mute = mute;
        self
    }

    /// set whether we're deafened
    #[inline]
    pub fn deaf(mut self, deaf: bool) -> Self {
        self.self_deaf = deaf;
        self
    }

    async fn connect(self) -> Result<Voice, VoiceError> {
        let channel_id = self.channel_id;

        let (tx, rx) = mpsc::channel::<RtcCommand>(64);

        let local_addr: SocketAddr = "0.0.0.0:0".parse().expect("always valid");
        let sock = UdpSocket::bind(local_addr).await?;

        let vs = VoiceStateUpdate {
            channel_id,
            self_mute: self.self_mute,
            self_deaf: self.self_deaf,
            self_video: false,
            screenshare: None,
        };
        let state = Arc::new(Shared {
            tx,
            sock,
            tracks: Default::default(),
            voice_state: watch::channel(LocalVoiceState::Connecting(vs.clone())).0,
            conn_state: watch::channel(ConnectionState::Pending).0,
        });

        let rtc = Rtc::builder().build(Instant::now());
        let worker = Driver::new(rtc, rx, self.client.syncer(), state.clone(), channel_id);
        tokio::spawn(worker.spawn());

        // find public ipv4 addr via stun
        tokio::spawn(state.clone().stun_v4());

        // TODO: ipv6 support

        self.client.syncer().send(MessageClient::VoiceConnect {
            voice_state: vs,
            nonce: None,
        });

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
