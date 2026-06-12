use crate::{state::MessageBroadcastInner, Result};
use common::v1::types::{voice::messages::SfuCommand, MessageSync, RoomId};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Sender;

type BoxStream<T> = std::pin::Pin<Box<dyn Stream<Item = T> + Send>>;

/// a message that can be broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Broadcast {
    /// a sync message
    Sync(BroadcastSync),

    /// a sfu command
    Sfu(SfuCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastSync {
    pub message: MessageSync,
    pub nonce: Option<String>,
}

impl Broadcast {
    pub fn sync(message: MessageSync) -> BroadcastSync {
        todo!()
    }
}

impl BroadcastSync {
    pub fn with_nonce(self, s: String) -> Self {
        todo!()
    }
}

impl From<MessageSync> for Broadcast {
    fn from(value: MessageSync) -> Self {
        todo!()
    }
}

impl From<BroadcastSync> for Broadcast {
    fn from(value: BroadcastSync) -> Self {
        todo!()
    }
}

pub struct Messaging {
    transport: Transport,
}

enum Transport {
    /// use tokio channels to broadcast events
    Memory {
        /// ALL events on the server
        sushi: Sender<MessageBroadcastInner>,

        /// ALL events for voice sfus
        sushi_sfu: Sender<SfuCommand>,
    },

    /// use nats to broadcast events
    Nats {
        client: async_nats::Client,

        /// ALL events on the server
        sushi: Sender<MessageBroadcastInner>,

        /// ALL events for voice sfus
        sushi_sfu: Sender<SfuCommand>,
    },
}

impl Messaging {
    /// send a message to everyone in a room
    pub async fn broadcast_room(
        &self,
        room_id: RoomId,
        broadcast: impl Into<Broadcast>,
    ) -> Result<()> {
        todo!()
    }

    // pub async fn broadcast_channel
    // pub async fn broadcast_user
    // pub async fn broadcast_global
    // async fn broadcast_inner

    /// subscribe to everything
    pub async fn subscribe(&self) -> Result<BoxStream<Broadcast>> {
        todo!()
    }
}
