use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use common::v1::types::{ChannelId, MessageId, RoomId, UserId};
use dashmap::DashMap;
use kameo::actor::Spawn;
use kameo::prelude::ActorRef;
use serenity::all::{
    ChannelId as DcChannelId, GuildId as DcGuildId, MessageId as DcMessageId, Presence,
    User as DcUser, UserId as DcUserId,
};
use tokio::sync::OnceCell;

use crate::bridge::Bridge;
use crate::bridge::BridgeMessage;
use crate::config::Config;
use crate::db::Data;
use crate::discord::Discord;
use crate::lamprey::Lamprey;
use crate::lamprey::LampreyHandle;
use crate::portal::{Portal, PortalMessage};

#[derive(Clone)]
pub struct UserCacheEntry {
    pub user: DcUser,
    pub fetched_at: Instant,
}

#[derive(Clone)]
pub struct Globals {
    pub pool: sqlx::SqlitePool,
    pub config: Config,
    pub portals: Arc<DashMap<ChannelId, ActorRef<Portal>>>,
    pub last_lamprey_ids: Arc<DashMap<ChannelId, MessageId>>,
    pub last_discord_ids: Arc<DashMap<DcChannelId, DcMessageId>>,
    pub presences: Arc<DashMap<DcUserId, Presence>>,
    pub discord_user_cache: Arc<DashMap<DcUserId, UserCacheEntry>>,
    pub discord: Arc<OnceCell<Discord>>,
    pub lamprey_chan: OnceCell<ActorRef<Lamprey>>,
    pub bridge_chan: OnceCell<ActorRef<Bridge>>,
    pub lamprey_user_id: Arc<OnceCell<UserId>>,
    pub recently_created_discord_channels: Arc<DashMap<DcChannelId, ()>>,
    pub reqwest_client: reqwest::Client,
}

impl Globals {
    pub fn new(pool: sqlx::SqlitePool, config: Config) -> Self {
        Self {
            pool,
            config,
            portals: Arc::new(DashMap::new()),
            last_lamprey_ids: Arc::new(DashMap::new()),
            last_discord_ids: Arc::new(DashMap::new()),
            presences: Arc::new(DashMap::new()),
            discord_user_cache: Arc::new(DashMap::new()),
            discord: Arc::new(OnceCell::new()),
            lamprey_chan: OnceCell::new(),
            bridge_chan: OnceCell::new(),
            lamprey_user_id: Arc::new(OnceCell::new()),
            recently_created_discord_channels: Arc::new(DashMap::new()),
            reqwest_client: reqwest::Client::new(),
        }
    }

    pub fn set_discord(&self, discord: Discord) -> Result<()> {
        self.discord
            .set(discord)
            .map_err(|_| anyhow::anyhow!("Discord already initialized"))?;
        Ok(())
    }

    pub fn get_discord(&self) -> Result<&Discord> {
        self.discord
            .get()
            .ok_or_else(|| anyhow::anyhow!("Discord not initialized"))
    }

    pub fn set_bridge_chan(&self, bridge_chan: ActorRef<Bridge>) -> Result<()> {
        self.bridge_chan
            .set(bridge_chan)
            .map_err(|_| anyhow::anyhow!("Bridge already initialized"))?;
        Ok(())
    }

    pub fn get_bridge_chan(&self) -> Result<ActorRef<Bridge>> {
        self.bridge_chan
            .get()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Bridge not initialized"))
    }

    pub fn set_lamprey_chan(&self, lamprey_chan: ActorRef<Lamprey>) -> Result<()> {
        self.lamprey_chan
            .set(lamprey_chan)
            .map_err(|_| anyhow::anyhow!("Lamprey already initialized"))?;
        Ok(())
    }

    pub fn get_lamprey_chan(&self) -> Result<ActorRef<Lamprey>> {
        self.lamprey_chan
            .get()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Lamprey not initialized"))
    }

    pub fn set_lamprey_user_id(&self, user_id: UserId) -> Result<()> {
        self.lamprey_user_id
            .set(user_id)
            .map_err(|_| anyhow::anyhow!("User ID already initialized"))?;
        Ok(())
    }

    pub fn get_lamprey_user_id(&self) -> Result<UserId> {
        self.lamprey_user_id
            .get()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("User ID not initialized"))
    }

    pub async fn lamprey_handle(&self) -> Result<LampreyHandle> {
        let lamprey_ref = self.get_lamprey_chan()?;
        Ok(LampreyHandle {
            lamprey_ref,
            globals: Arc::new(self.clone()),
        })
    }
}

/// defines a single chatroom bridged together
#[derive(Debug, Clone)]
pub struct PortalConfig {
    pub lamprey_thread_id: ChannelId,
    pub lamprey_room_id: RoomId,
    pub discord_guild_id: DcGuildId,
    // TODO: make discord_channel_id the thread id if the target is a thread, and add this field
    // pub discord_webhook_channel_id: DcChannelId, // the thread's parent channel if it exists
    pub discord_channel_id: DcChannelId,
    pub discord_thread_id: Option<DcChannelId>,
    pub discord_webhook: String,
}

/// defines a collection of chatrooms bridged together (eg. discord guilds and lamprey rooms)
#[derive(Debug, Clone)]
pub struct RealmConfig {
    pub lamprey_room_id: RoomId,
    pub discord_guild_id: DcGuildId,

    /// if new portals are automatically created when a discord channel or lamprey thread is created
    pub continuous: bool,
}

#[async_trait]
pub trait GlobalsTrait {
    async fn portal_send(&self, thread_id: ChannelId, msg: PortalMessage);
    async fn portal_send_dc(&self, channel_id: DcChannelId, msg: PortalMessage);
}

#[async_trait]
impl GlobalsTrait for Arc<Globals> {
    async fn portal_send(&self, thread_id: ChannelId, msg: PortalMessage) {
        let Ok(Some(config)) = self.get_portal_by_thread_id(thread_id).await else {
            return;
        };
        let portal_ref = self
            .portals
            .entry(config.lamprey_thread_id)
            .or_insert_with(|| Portal::spawn((self.clone(), config.to_owned())));
        let _ = portal_ref.tell(msg).await;
    }

    async fn portal_send_dc(&self, channel_id: DcChannelId, msg: PortalMessage) {
        let Ok(Some(config)) = self.get_portal_by_discord_channel(channel_id).await else {
            return;
        };
        let portal_ref = self
            .portals
            .entry(config.lamprey_thread_id)
            .or_insert_with(|| Portal::spawn((self.clone(), config.to_owned())));
        let _ = portal_ref.tell(msg).await;
    }
}

impl Globals {
    pub async fn bridge_send(&self, msg: BridgeMessage) -> Result<()> {
        if let Ok(bridge_chan) = self.get_bridge_chan() {
            if let Err(e) = bridge_chan.tell(msg).await {
                tracing::error!("Failed to send message to Bridge actor: {}", e);
            }
        }
        Ok(())
    }
}

pub const WEBHOOK_NAME: &'static str = "bridg";
