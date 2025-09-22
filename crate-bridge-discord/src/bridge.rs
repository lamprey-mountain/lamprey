use std::sync::Arc;

use anyhow::Result;
use common::v1::types::{RoomId, ThreadId};
use serenity::all::{ChannelId as DcChannelId, GuildId as DcGuildId};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::{
    common::{Globals, PortalConfig},
    data::Data,
    discord,
    portal::Portal,
};

pub struct Bridge {
    globals: Arc<Globals>,
    recv: mpsc::UnboundedReceiver<BridgeMessage>,
}

#[derive(Debug, Clone)]
pub enum BridgeMessage {
    LampreyThreadCreate {
        thread_id: ThreadId,
        room_id: RoomId,
        thread_name: String,
        discord_guild_id: DcGuildId,
    },
    DiscordChannelCreate {
        guild_id: DcGuildId,
        channel_id: DcChannelId,
        channel_name: String,
    },
}

impl Bridge {
    pub fn spawn(globals: Arc<Globals>, recv: mpsc::UnboundedReceiver<BridgeMessage>) {
        let bridge = Self { globals, recv };
        tokio::spawn(bridge.activate());
    }

    async fn activate(mut self) {
        while let Some(msg) = self.recv.recv().await {
            if let Err(err) = self.handle(msg).await {
                error!("{err}")
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn handle(&mut self, msg: BridgeMessage) -> Result<()> {
        match msg {
            BridgeMessage::LampreyThreadCreate {
                thread_id,
                room_id,
                thread_name,
                discord_guild_id,
            } => {
                if self
                    .globals
                    .get_portal_by_thread_id(thread_id)
                    .await
                    .is_ok_and(|a| a.is_some())
                {
                    info!("portal already exists");
                    return Ok(());
                }

                info!("autobridging thread {}", thread_id);
                let name = if thread_name.is_empty() {
                    "thread".to_string()
                } else {
                    thread_name
                };
                let channel_id = discord::discord_create_channel(
                    self.globals.clone(),
                    discord_guild_id,
                    name.clone(),
                )
                .await?;
                let webhook = discord::discord_create_webhook(
                    self.globals.clone(),
                    channel_id,
                    "bridge".to_string(),
                )
                .await?;
                let portal = PortalConfig {
                    lamprey_thread_id: thread_id,
                    lamprey_room_id: room_id,
                    discord_guild_id,
                    discord_channel_id: channel_id,
                    discord_thread_id: None,
                    discord_webhook: webhook.url().unwrap().to_string(),
                };
                self.globals.insert_portal(portal.clone()).await?;
                self.globals
                    .portals
                    .entry(portal.lamprey_thread_id)
                    .or_insert_with(|| Portal::summon(self.globals.clone(), portal));
            }
            BridgeMessage::DiscordChannelCreate {
                guild_id,
                channel_id,
                channel_name,
            } => {
                let Ok(realms) = self.globals.get_realms().await else {
                    return Ok(());
                };

                let Some(realm_config) = realms.iter().find(|c| c.discord_guild_id == guild_id)
                else {
                    return Ok(());
                };

                if !realm_config.continuous {
                    return Ok(());
                }

                if self
                    .globals
                    .get_portal_by_discord_channel(channel_id)
                    .await
                    .is_ok_and(|a| a.is_some())
                {
                    info!("already exists");
                    return Ok(());
                }

                info!("autobridging discord channel {}", channel_id);
                let ly = self.globals.lamprey_handle().await?;

                let thread_name = if channel_name.is_empty() {
                    "thread".to_string()
                } else {
                    channel_name.clone()
                };

                let thread = ly
                    .create_thread(realm_config.lamprey_room_id, thread_name.clone(), None)
                    .await?;

                let webhook = discord::discord_create_webhook(
                    self.globals.clone(),
                    channel_id,
                    "bridge".to_string(),
                )
                .await?;

                let portal_config = PortalConfig {
                    lamprey_thread_id: thread.id,
                    lamprey_room_id: realm_config.lamprey_room_id,
                    discord_guild_id: guild_id,
                    discord_channel_id: channel_id,
                    discord_thread_id: None,
                    discord_webhook: webhook.url().unwrap().to_string(),
                };

                self.globals.insert_portal(portal_config.clone()).await?;

                self.globals
                    .portals
                    .entry(portal_config.lamprey_thread_id)
                    .or_insert_with(|| Portal::summon(self.globals.clone(), portal_config));
            }
        }

        Ok(())
    }
}
