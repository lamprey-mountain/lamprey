use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use common::v1::types::ThreadId;
use dashmap::DashMap;
use serde::Deserialize;
use serenity::all::ChannelId as DcChannelId;
use tokio::sync::{mpsc, oneshot};

use crate::data::{Data, MessageMetadata};
use crate::lamprey::LampreyHandle;
use crate::portal::{Portal, PortalMessage};
use crate::{discord::DiscordMessage, lamprey::LampreyMessage};

#[derive(Clone)]
pub struct Globals {
    pub pool: sqlx::SqlitePool,
    pub config: Config,
    pub portals: Arc<DashMap<ThreadId, mpsc::UnboundedSender<PortalMessage>>>,
    pub last_ids: Arc<DashMap<ThreadId, MessageMetadata>>,
    pub dc_chan: mpsc::Sender<DiscordMessage>,
    pub(super) ch_chan: mpsc::Sender<LampreyMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub lamprey_token: String,
    pub lamprey_base_url: Option<String>,
    pub lamprey_ws_url: Option<String>,
    pub discord_token: String,
}

#[async_trait]
pub trait GlobalsTrait {
    async fn portal_send(&self, thread_id: ThreadId, msg: PortalMessage);
    async fn portal_send_dc(&self, channel_id: DcChannelId, msg: PortalMessage);
}

#[async_trait]
impl GlobalsTrait for Arc<Globals> {
    async fn portal_send(&self, thread_id: ThreadId, msg: PortalMessage) {
        let Ok(Some(config)) = self.get_portal_by_thread_id(thread_id).await else {
            return;
        };
        let portal = self
            .portals
            .entry(config.lamprey_thread_id)
            .or_insert_with(|| Portal::summon(self.clone(), config.to_owned()));
        let _ = portal.send(msg);
    }

    async fn portal_send_dc(&self, channel_id: DcChannelId, msg: PortalMessage) {
        let Ok(Some(config)) = self.get_portal_by_discord_channel(channel_id).await else {
            return;
        };
        let portal = self
            .portals
            .entry(config.lamprey_thread_id)
            .or_insert_with(|| Portal::summon(self.clone(), config.to_owned()));
        let _ = portal.send(msg);
    }
}

impl Globals {
    pub async fn lamprey_handle(&self) -> Result<LampreyHandle> {
        let (send, recv) = oneshot::channel();
        self.ch_chan
            .send(LampreyMessage::Handle { response: send })
            .await?;
        Ok(recv.await?)
    }
}
