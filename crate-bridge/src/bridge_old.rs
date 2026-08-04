use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{
    sync::{broadcast, oneshot},
    task::JoinHandle,
};
use url::Url;

use crate::{database::Database, prelude::*};

// TEMP: compat
use crate::actor::bridge::BridgeCommand;
pub use crate::types::*;

/// a set of portals
///
/// can automatically create/delete portals as channels are created/deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Realm {
    pub id: RealmId,
    pub continuous: bool,
}

/// a single logical channel. forwards messages across platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub id: PortalId,
    pub realm_id: Option<RealmId>,
    pub lamprey: Option<PortalLamprey>,
    pub discord: Option<PortalDiscord>,
    // pub lamprey: PlatformState,
    // pub discord: PlatformState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalLamprey {
    pub channel_id: lamprey::ChannelId,
    pub room_id: lamprey::RoomId,
    pub last_id: lamprey::MessageId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalDiscord {
    pub guild_id: discord::GuildId,
    pub parent_id: Option<discord::ChannelId>, // for threads
    pub channel_id: discord::ChannelId,
    pub webhook_url: Url,
    pub last_id: discord::MessageId,
}

// TEMP: reexport
pub use crate::actor::bridge::BridgeEvent;

// TODO: remove?
#[derive(Debug, Clone)]
pub struct PortalCreate {
    pub realm_id: Option<RealmId>,
    pub source_platform: Platform,
    pub source_id: String,
    pub channel: PortalChannel,
}

/// an event that's broadcast to a portal
#[derive(Debug, Clone)]
pub enum PortalEvent {
    Typing(User),

    MessageCreate(MessageData),
    MessageUpdate(MessageData),
    MessageDelete(MessageId),

    ReactionCreate(MessageId, String, User),
    ReactionDelete(MessageId, String, User),
    ReactionDeleteEmoji(MessageId, String),
    ReactionDeleteAll(MessageId, String),
}

// TODO: portal actor
// #[derive(Debug, Clone)]
// pub struct PortalActor {}
//
// pub enum PortalCommand {
//     // Typing(User),
//     // MessageCreate(MessageData),
//     // MessageUpdate(MessageData),
//     // MessageDelete(MessageId),
//     // etc...
// }

#[derive(Debug, Clone)]
pub struct BridgeHandle {
    pub commands: tokio::sync::mpsc::Sender<BridgeCommand>,
    pub events: broadcast::Sender<Arc<BridgeEvent>>,
    pub db: Arc<dyn Database>,
}

#[derive(Debug, Clone)]
pub struct PortalHandle {
    pub id: PortalId,
    pub events: broadcast::Sender<Arc<PortalEvent>>,
    pub bridge: BridgeHandle,
}

pub struct PlatformHandle {
    pub name: &'static str,
    pub ready: oneshot::Receiver<()>,
    pub task: JoinHandle<Result<()>>,
}

// TODO: make this configurable?
pub const BROADCAST_CHANNEL_CAPACITY: usize = 1024;

impl BridgeHandle {
    pub fn new(db: Arc<dyn Database>, commands: tokio::sync::mpsc::Sender<BridgeCommand>) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self {
            commands,
            events,
            db,
        }
    }

    pub fn create_portal_handle(&self, id: PortalId) -> PortalHandle {
        PortalHandle::new(id, self.clone())
    }

    // pub fn create_realm_handle(&self, id: RealmId) -> RealmHandle {
    //     RealmHandle::new(id, self.clone())
    // }
}

impl PortalHandle {
    pub fn new(id: PortalId, bridge: BridgeHandle) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self { id, events, bridge }
    }
}
