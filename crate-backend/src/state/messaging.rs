use crate::prelude::*;
use common::v1::types::{voice::messages::SfuCommand, ChannelId, MessageSync, RoomId, UserId};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{self, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tracing::error;

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
        BroadcastSync {
            message,
            nonce: None,
        }
    }
}

impl BroadcastSync {
    pub fn with_nonce(self, s: String) -> Self {
        Self {
            nonce: Some(s),
            ..self
        }
    }
}

impl From<MessageSync> for Broadcast {
    fn from(value: MessageSync) -> Self {
        Broadcast::Sync(BroadcastSync {
            message: value,
            nonce: None,
        })
    }
}

impl From<BroadcastSync> for Broadcast {
    fn from(value: BroadcastSync) -> Self {
        Broadcast::Sync(value)
    }
}

impl From<SfuCommand> for Broadcast {
    fn from(value: SfuCommand) -> Self {
        Broadcast::Sfu(value)
    }
}

#[derive(Clone)]
pub struct Messaging {
    transport: Transport,
}

#[derive(Clone)]
pub enum Transport {
    /// use tokio channels to broadcast events
    Memory {
        /// ALL events on the server
        sushi: Sender<BroadcastSync>,

        /// ALL events for voice sfus
        sushi_sfu: Sender<SfuCommand>,
    },

    /// use nats to broadcast events
    Nats {
        client: async_nats::Client,

        /// ALL events on the server
        sushi: Sender<BroadcastSync>,

        /// ALL events for voice sfus
        sushi_sfu: Sender<SfuCommand>,
    },
}

impl Transport {
    /// create a new in memory transport
    pub fn memory() -> Self {
        let (sushi, _) = broadcast::channel::<BroadcastSync>(100);
        let (sushi_sfu, _) = broadcast::channel::<SfuCommand>(100);
        Self::Memory { sushi, sushi_sfu }
    }

    /// create a new nats transport
    pub fn nats(client: async_nats::Client) -> Self {
        let (sushi, _) = broadcast::channel::<BroadcastSync>(100);
        let (sushi_sfu, _) = broadcast::channel::<SfuCommand>(100);

        // forward nats events to local tokio broadcast channels
        fn spawn_forwarder<T>(client: async_nats::Client, subject: &str, tx: Sender<T>)
        where
            T: for<'de> Deserialize<'de> + Send + 'static,
        {
            let subject = subject.to_string();
            tokio::spawn(async move {
                let mut sub = match client.subscribe(subject.clone()).await {
                    Ok(sub) => sub,
                    Err(e) => {
                        error!("failed to subscribe to NATS '{subject}': {e}");
                        return;
                    }
                };
                while let Some(msg) = sub.next().await {
                    if let Ok(m) = serde_json::from_slice::<T>(&msg.payload) {
                        let _ = tx.send(m);
                    }
                }
            });
        }

        spawn_forwarder(client.clone(), "sushi", sushi.clone());
        spawn_forwarder(client.clone(), "sushi_sfu", sushi_sfu.clone());

        Self::Nats {
            client,
            sushi,
            sushi_sfu,
        }
    }
}

impl Messaging {
    pub fn new(transport: Transport) -> Self {
        Self { transport }
    }

    pub fn is_connected(&self) -> bool {
        match &self.transport {
            Transport::Memory { .. } => true,
            Transport::Nats { client, .. } => {
                client.connection_state() == async_nats::connection::State::Connected
            }
        }
    }

    /// send a message to everyone in a room
    pub async fn broadcast_room(
        &self,
        _room_id: RoomId,
        broadcast: impl Into<Broadcast>,
    ) -> Result<()> {
        self.broadcast_inner(broadcast.into()).await
    }

    pub async fn broadcast_channel(
        &self,
        _channel_id: ChannelId,
        broadcast: impl Into<Broadcast>,
    ) -> Result<()> {
        self.broadcast_inner(broadcast.into()).await
    }

    pub async fn broadcast_user(
        &self,
        _user_id: UserId,
        broadcast: impl Into<Broadcast>,
    ) -> Result<()> {
        self.broadcast_inner(broadcast.into()).await
    }

    pub async fn broadcast_global(&self, broadcast: impl Into<Broadcast>) -> Result<()> {
        self.broadcast_inner(broadcast.into()).await
    }

    async fn broadcast_inner(&self, broadcast: Broadcast) -> Result<()> {
        match &self.transport {
            Transport::Memory { sushi, sushi_sfu } => match broadcast {
                Broadcast::Sync(s) => {
                    let _ = sushi.send(s);
                }
                Broadcast::Sfu(c) => {
                    let _ = sushi_sfu.send(c);
                }
            },
            Transport::Nats { client, .. } => match broadcast {
                Broadcast::Sync(s) => {
                    let bytes = serde_json::to_vec(&s)?;
                    client.publish("sushi".to_string(), bytes.into()).await?;
                }
                Broadcast::Sfu(c) => {
                    let bytes = serde_json::to_vec(&c)?;
                    client
                        .publish("sushi_sfu".to_string(), bytes.into())
                        .await?;
                }
            },
        }
        Ok(())
    }

    /// subscribe to everything
    pub async fn subscribe(&self) -> Result<BoxStream<Broadcast>> {
        match &self.transport {
            Transport::Memory { sushi, sushi_sfu }
            | Transport::Nats {
                sushi, sushi_sfu, ..
            } => {
                let sushi_stream = BroadcastStream::new(sushi.subscribe())
                    .filter_map(|res| async move { res.ok().map(Broadcast::Sync) });

                let sfu_stream = BroadcastStream::new(sushi_sfu.subscribe())
                    .filter_map(|res| async move { res.ok().map(Broadcast::Sfu) });

                Ok(Box::pin(futures::stream::select(sushi_stream, sfu_stream)))
            }
        }
    }
}
