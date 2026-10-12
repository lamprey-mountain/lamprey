use std::sync::Arc;

use common::{v1::types::voice::VoiceState, v2::types::ChannelId};
use futures_util::{StreamExt, stream::BoxStream};
use tokio::sync::{broadcast, mpsc};

use crate::voice::{VoiceError, VoiceEvent, driver::RtcCommand};

/// a connection to a voice channel
#[derive(Clone)]
pub struct Voice {
    pub(crate) state: Arc<Shared>,
}

pub(crate) struct Shared {
    pub(crate) tx: mpsc::Sender<RtcCommand>,
    // rx: broadcast::Sender<RtcEvent>,
}

/// tracks that are known to the client
pub struct Tracks {
    // ...
    // maybe store in VoiceInner?
    // fn publish(track)
}

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

impl Voice {
    /// get the id we're connected to
    pub fn channel_id(&self) -> ChannelId {
        todo!()
    }

    /// get the current voice state
    pub fn voice_state(&self) -> &VoiceState {
        todo!()
    }

    /// get the current connection state
    pub fn connection_state(&self) -> ConnectionState {
        todo!()
    }

    /// access the track registry
    pub fn tracks(&self) -> Tracks {
        todo!()
    }

    // /// get a stream of events
    // pub fn events(&self) -> BoxStream<'static, VoiceEvent> {
    //     let rx = self.state.rx.subscribe();
    //     let rx = tokio_stream::wrappers::BroadcastStream::new(rx);
    //     rx.filter_map(|evt| async move {
    //         match evt {
    //             Ok(RtcEvent::Signalling(_cmd)) => {
    //                 // FIXME: Map RtcEvent to VoiceEvent
    //                 None
    //             }
    //             Err(_) => None,
    //         }
    //     })
    //     .boxed()
    // }

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

    // /// create a new datachannel
    // pub async fn create_channel(&self, _protocol: ProtocolType) -> Result<(), VoiceError> {
    //     todo!()
    // }

    // /// move to a different channel
    // ///
    // /// will attempt to recreate all existing tracks
    // pub async fn move_channel(&self, _channel_id: ChannelId) -> Result<(), VoiceError> {
    //     todo!()
    // }

    pub async fn mute(&self, _mute: bool) -> Result<(), VoiceError> {
        todo!()
    }

    pub async fn deaf(&self, _deaf: bool) -> Result<(), VoiceError> {
        todo!()
    }

    // // TODO: use speaking flags
    // pub async fn speaking(&self, _speaking: bool) -> Result<(), VoiceError> {
    //     todo!()
    // }

    pub async fn disconnect(self) -> Result<(), VoiceError> {
        todo!()
    }

    pub fn is_mute(&self) -> bool {
        todo!()
    }

    pub fn is_deaf(&self) -> bool {
        todo!()
    }
}
