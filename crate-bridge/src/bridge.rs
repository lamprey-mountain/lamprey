use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use url::Url;

use crate::{database::Database, prelude::*};

// TODO: use newtypes instead of type aliases
pub type PortalId = u32;
pub type RealmId = u32;
pub type MessageId = u32;

/// a set of portals
///
/// can automatically create/delete portals as channels are created/deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Realm {
    pub continuous: bool,
}

/// a single logical channel. forwards messages across platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub realm_id: Option<RealmId>,
    pub lamprey: Option<PortalLamprey>,
    pub discord: Option<PortalDiscord>,
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
/// metadata for a single logical message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub portal_id: PortalId,
    pub source_platform: Platform,

    pub lamprey_message_id: Option<lamprey::MessageId>,
    pub discord_message_id: Option<discord::MessageId>,
    /// media/attachment ids to know what needs to be uploaded on edit vs what can be reused
    pub attachments: Vec<(lamprey::MediaId, discord::AttachmentId)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, strum::Display, strum::EnumString, PartialEq, Eq)]
pub enum Platform {
    Lamprey,
    Discord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub source_platform: Platform,
    pub lamprey_id: lamprey::UserId,
    pub discord_id: discord::UserId,

    // used for syncing media
    pub discord_avatar_url: Option<String>,
    pub discord_banner_url: Option<String>,
}

/// an event that's broadcast to a bridge
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// load a realm from the database
    RealmInit(Realm),

    /// load a portal from the database
    PortalInit(PortalId, Portal, PortalHandle),

    /// an event for a portal
    PortalEvent(PortalId, PortalEvent),

    /// a portal should be deleted
    ///
    /// the sender of this event should delete stuff from the database
    PortalDeleted(PortalId),

    /// a portal has been requested to be created
    PortalRequest(PortalId, PortalCreate),
    // TODO: more events
    // PortalUpdate,
    // UserUpdate,
    // MemberCreate,
    // MemberUpdate,
    // MemberDelete,
    // PresenceUpdate,

    // UserUpdate {
    //     source_user_id: UserId,
    //     avatar_url: Option<Url>,
    //     banner_url: Option<Url>,
    // },
}

#[derive(Debug, Clone)]
pub struct PortalCreate {
    pub realm_id: RealmId,
    pub source_platform: Platform,
    pub source_id: String,
    pub channel: PortalChannel,
}

#[derive(Debug, Clone)]
pub struct PortalChannel {
    pub name: String,
    pub description: String,
    pub kind: ChannelKind,
    pub parent_id: Option<PortalId>,
    pub position: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum ChannelKind {
    Text,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum MessageData {
    Lamprey(lamprey::Message),
    Discord(discord::Message),
}

impl MessageData {
    // TODO
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

#[derive(Debug, Clone)]
pub struct BridgeHandle {
    pub events: broadcast::Sender<Arc<BridgeEvent>>,
    pub db: Arc<dyn Database>,
}

#[derive(Debug, Clone)]
pub struct PortalHandle {
    pub id: PortalId,
    pub events: broadcast::Sender<Arc<PortalEvent>>,
    pub bridge: BridgeHandle,
}

pub const BROADCAST_CHANNEL_CAPACITY: usize = 1024;

impl BridgeHandle {
    pub fn new(db: Arc<dyn Database>) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self { events, db }
    }

    pub fn create_portal_handle(&self, id: PortalId) -> PortalHandle {
        PortalHandle::new(id, self.clone())
    }
}

impl PortalHandle {
    pub fn new(id: PortalId, bridge: BridgeHandle) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        Self { id, events, bridge }
    }
}
