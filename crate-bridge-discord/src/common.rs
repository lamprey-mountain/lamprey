use std::sync::Arc;

use common::v1::types::ThreadId;
use dashmap::DashMap;
use serde::Deserialize;
use serenity::all::{ChannelId as DcChannelId, GuildId as DcGuildId};
use tokio::sync::mpsc;

use crate::data::MessageMetadata;
use crate::portal::{Portal, PortalMessage};
use crate::{chat::UnnamedMessage, discord::DiscordMessage};

#[derive(Clone)]
pub struct Globals {
    pub pool: sqlx::SqlitePool,
    pub config: Config,
    pub portals: Arc<DashMap<ThreadId, mpsc::UnboundedSender<PortalMessage>>>,
    pub last_ids: Arc<DashMap<ThreadId, MessageMetadata>>,
    pub dc_chan: mpsc::Sender<DiscordMessage>,
    pub ch_chan: mpsc::Sender<UnnamedMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub portal: Vec<ConfigPortal>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigPortal {
    pub my_thread_id: ThreadId,
    pub discord_guild_id: DcGuildId,
    pub discord_channel_id: DcChannelId,
    pub discord_thread_id: Option<DcChannelId>,
    pub discord_webhook: String,
}

impl ConfigPortal {
    #[inline]
    pub fn discord_channel_or_thread_id(&self) -> DcChannelId {
        self.discord_thread_id.unwrap_or(self.discord_channel_id)
    }
}

impl Config {
    pub fn portal_by_discord_id(&self, id: DcChannelId) -> Option<&ConfigPortal> {
        self.portal
            .iter()
            .find(|i| i.discord_channel_or_thread_id() == id)
    }

    pub fn portal_by_thread_id(&self, id: ThreadId) -> Option<&ConfigPortal> {
        self.portal.iter().find(|i| i.my_thread_id == id)
    }
}

pub trait GlobalsTrait {
    fn portal_send(&mut self, thread_id: ThreadId, msg: PortalMessage);
    fn portal_send_dc(&mut self, channel_id: DcChannelId, msg: PortalMessage);
}

impl GlobalsTrait for Arc<Globals> {
    fn portal_send(&mut self, thread_id: ThreadId, msg: PortalMessage) {
        let Some(config) = self.config.portal_by_thread_id(thread_id) else {
            return;
        };
        let portal = self
            .portals
            .entry(config.my_thread_id)
            .or_insert_with(|| Portal::summon(self.clone(), config.to_owned()));
        let _ = portal.send(msg);
    }

    fn portal_send_dc(&mut self, channel_id: DcChannelId, msg: PortalMessage) {
        let Some(config) = self.config.portal_by_discord_id(channel_id) else {
            return;
        };
        let portal = self
            .portals
            .entry(config.my_thread_id)
            .or_insert_with(|| Portal::summon(self.clone(), config.to_owned()));
        let _ = portal.send(msg);
    }
}
