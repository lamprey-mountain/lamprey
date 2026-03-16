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
use tokio::sync::RwLock;

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
    pub discord: Arc<RwLock<Option<Discord>>>,
    pub ch_chan: Arc<RwLock<Option<ActorRef<Lamprey>>>>,
    pub bridge_chan: Arc<RwLock<Option<ActorRef<Bridge>>>>,
    pub lamprey_user_id: Arc<RwLock<Option<UserId>>>,
    pub recently_created_discord_channels: Arc<DashMap<DcChannelId, ()>>,
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
            discord: Arc::new(RwLock::new(None)),
            ch_chan: Arc::new(RwLock::new(None)),
            bridge_chan: Arc::new(RwLock::new(None)),
            lamprey_user_id: Arc::new(RwLock::new(None)),
            recently_created_discord_channels: Arc::new(DashMap::new()),
        }
    }

    pub async fn set_discord(&self, discord: Discord) {
        *self.discord.write().await = Some(discord);
    }

    pub async fn take_discord(&self) -> Option<Discord> {
        self.discord.write().await.take()
    }

    pub async fn with_discord<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Discord) -> R,
    {
        let mut guard = self.discord.write().await;
        guard.as_mut().map(f)
    }

    pub async fn set_bridge_chan(&self, bridge_chan: ActorRef<Bridge>) {
        *self.bridge_chan.write().await = Some(bridge_chan);
    }

    pub async fn get_bridge_chan(&self) -> Option<ActorRef<Bridge>> {
        self.bridge_chan.read().await.clone()
    }

    pub async fn set_lamprey_chan(&self, lamprey_chan: ActorRef<Lamprey>) {
        *self.ch_chan.write().await = Some(lamprey_chan);
    }

    pub async fn get_lamprey_chan(&self) -> Option<ActorRef<Lamprey>> {
        self.ch_chan.read().await.clone()
    }

    pub async fn lamprey_handle(&self) -> Result<LampreyHandle> {
        let Some(lamprey_ref) = self.get_lamprey_chan().await else {
            return Err(anyhow::anyhow!("lamprey actor not initialized"));
        };
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
        if let Some(bridge_chan) = self.get_bridge_chan().await {
            let _ = bridge_chan.tell(msg).await;
        }
        Ok(())
    }
}

pub const WEBHOOK_NAME: &'static str = "bridg";
