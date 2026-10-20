use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use common::{
    v1::types::voice::{Mid, TrackId, VoiceState, VoiceStateUpdate},
    v2::types::ChannelId,
};
use dashmap::DashMap;
use str0m::Candidate;
use stunclient::StunClient;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, watch},
};

use crate::voice::{
    VoiceError,
    driver::RtcCommand,
    player::{Producer, PublishBuilder},
    track::{Inbound, Outbound, Track},
};

/// a connection to a voice channel
#[derive(Clone)]
pub struct Voice {
    pub(super) state: Arc<Shared>,
}

pub(super) struct Shared {
    // TODO: use atomic for ConnectionState
    pub(super) tx: mpsc::Sender<RtcCommand>,
    pub(super) sock: UdpSocket,
    pub(super) tracks: Tracks,
    // TODO: store call participants (voice states)
    // TODO: store datachannels
    pub(super) voice_state: watch::Sender<LocalVoiceState>,

    /// the current connection state
    pub(super) conn_state: watch::Sender<ConnectionState>,
}

// NOTE: this could be done with a struct?
// desired: VoiceStateUpdate, confirmed: Option<VoiceState>
pub(super) enum LocalVoiceState {
    Connecting(VoiceStateUpdate),
    Pending(VoiceState, VoiceStateUpdate),
    Synced(VoiceState),
}

/// media track registry
///
/// stores all tracks that the client knows about
#[derive(Debug, Default)]
pub struct Tracks {
    pub(super) inbound: DashMap<TrackId, Inbound>,
    pub(super) outbound: DashMap<Mid, Outbound>,
}

/// the current state of a voice connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// disconnected
    Disconnected,

    /// sent a VoiceState update, waiting for an sfu to connect to
    Pending,

    /// checking routes via ice
    Checking,

    /// connected to the sfu, ice still in progress
    Connected,

    /// fully connected to the sfu
    Ready,

    /// could not connect to the sfu
    // TODO: granular errors - is this an ice error, permission error, etc?
    // this is probably not fatal
    Failed,
}

// TODO: add a way to get some events from the client? receiving new inbound tracks, etc...
impl Voice {
    /// Get the ID of the channel we're connected to.
    pub fn channel_id(&self) -> ChannelId {
        self.state.voice_state.borrow().channel_id()
    }

    /// Get our current voice state according to the server.
    pub fn voice_state(&self) -> Option<VoiceState> {
        // PERF: use Arc for VoiceState?
        match &*self.state.voice_state.borrow() {
            LocalVoiceState::Connecting(_) => None,
            LocalVoiceState::Pending(s, _) => Some(s.clone()),
            LocalVoiceState::Synced(s) => Some(s.clone()),
        }
    }

    /// get the current connection state
    pub fn connection_state(&self) -> ConnectionState {
        *self.state.conn_state.borrow()
    }

    /// access the track registry
    pub fn tracks(&self) -> &Tracks {
        &self.state.tracks
    }

    // /// get a stream of incoming tracks
    // pub fn inbound(&self) -> BoxStream<'static, Inbound> {
    //     futures_util::stream::empty().boxed()
    // }

    // // TODO: use speaking flags
    // pub async fn speaking(&self, _speaking: bool) -> Result<(), VoiceError> {
    //     todo!()
    // }

    /// set whether we're muted
    pub async fn mute(&self, mute: bool) -> Result<(), VoiceError> {
        self.state
            .tx
            .send(RtcCommand::Mute(mute))
            .await
            .map_err(|_| VoiceError::ConnectionClosed)
    }

    /// set whether we're deafened
    pub async fn deaf(&self, deaf: bool) -> Result<(), VoiceError> {
        self.state
            .tx
            .send(RtcCommand::Deafen(deaf))
            .await
            .map_err(|_| VoiceError::ConnectionClosed)
    }

    /// move to a different channel
    ///
    /// this will attempt to recreate all existing outbound tracks into the new channel
    pub async fn move_to(&self, _channel_id: ChannelId) -> Result<(), VoiceError> {
        todo!()
    }

    /// disconnect from the voice channel
    ///
    /// after this is called, this voice session is dead and should not be reused.
    pub async fn disconnect(&self) -> Result<(), VoiceError> {
        self.state
            .tx
            .send(RtcCommand::Disconnect)
            .await
            .map_err(|_| VoiceError::ConnectionClosed)
    }

    /// returns whether the voice connection is currently muted
    pub fn is_mute(&self) -> bool {
        self.state.voice_state.borrow().mute()
    }

    /// returns whether the voice connection is currently deafened
    pub fn is_deaf(&self) -> bool {
        self.state.voice_state.borrow().deaf()
    }

    /// publish an outbound media track
    pub fn publish<P: Producer>(&self, prod: P) -> PublishBuilder<'_> {
        PublishBuilder::new(self, prod)
    }
}

impl Shared {
    /// perform a STUN query for an IPv4 address
    pub async fn stun_v4(self: Arc<Self>) -> Result<(), VoiceError> {
        self.stun_with(|s| s.is_ipv4()).await
    }

    /// perform a STUN query, filtering for a specific stun address
    pub async fn stun_with<F: Fn(&SocketAddr) -> bool>(
        self: Arc<Self>,
        filter: F,
    ) -> Result<(), VoiceError> {
        // TODO: allow configuring stun addr
        let stun_addr = "stun.l.google.com:19302"
            .to_socket_addrs()?
            .filter(filter)
            .next();

        if let Some(stun_addr) = stun_addr {
            let c = StunClient::new(stun_addr);
            let addr = c.query_external_address_async(&self.sock).await?;
            let candidate = Candidate::server_reflexive(addr, self.sock.local_addr()?, "udp")?;
            let _ = self.tx.send(RtcCommand::Candidate(candidate)).await;
        } else {
            // TODO: maybe warn!() via tracing?
        }

        Result::<(), VoiceError>::Ok(())
    }
}

impl Tracks {
    /// iterate over all inbound tracks
    pub fn inbound(&self) -> impl Iterator<Item = Inbound> {
        self.inbound.iter().map(|kv| kv.value().clone())
    }

    /// iterate over all outbound tracks
    pub fn outbound(&self) -> impl Iterator<Item = Outbound> {
        self.outbound.iter().map(|kv| kv.value().clone())
    }

    /// get a track by its server assigned track id
    ///
    /// track ids are the same for all users
    pub fn get(&self, track_id: TrackId) -> Option<Track> {
        // TODO: also lookup outbound tracks
        self.inbound
            .get(&track_id)
            .map(|t| Track::Inbound(t.value().clone()))
    }

    /// get a track by its media id
    ///
    /// mids are unique to each webrtc session
    pub fn get_by_mid(&self, mid: Mid) -> Option<Track> {
        // TODO: also lookup inbound tracks
        self.outbound
            .get(&mid)
            .map(|t| Track::Outbound(t.value().clone()))
    }
}

impl LocalVoiceState {
    /// did we mute ourselves?
    pub fn mute(&self) -> bool {
        match self {
            LocalVoiceState::Connecting(u) => u.self_mute,
            LocalVoiceState::Pending(_s, u) => u.self_mute,
            LocalVoiceState::Synced(s) => s.self_mute,
        }
    }

    /// did we deafen ourselves?
    pub fn deaf(&self) -> bool {
        match self {
            LocalVoiceState::Connecting(u) => u.self_deaf,
            LocalVoiceState::Pending(_s, u) => u.self_deaf,
            LocalVoiceState::Synced(s) => s.self_deaf,
        }
    }

    /// get the channel we are connected to
    pub fn channel_id(&self) -> ChannelId {
        match self {
            LocalVoiceState::Connecting(u) => u.channel_id,
            LocalVoiceState::Pending(_s, u) => u.channel_id,
            LocalVoiceState::Synced(s) => s.channel_id,
        }
    }
}
