use std::sync::Arc;

use anyhow::{anyhow, Result};
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
        thread: common::v1::types::Thread,
        discord_guild_id: DcGuildId,
    },
    DiscordChannelCreate {
        guild_id: DcGuildId,
        channel_id: DcChannelId,
        channel_name: String,
        channel_type: serenity::all::ChannelType,
        parent_id: Option<DcChannelId>,
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
                thread,
                discord_guild_id,
            } => {
                if self
                    .globals
                    .get_portal_by_thread_id(thread.id)
                    .await
                    .is_ok_and(|a| a.is_some())
                {
                    info!("portal already exists");
                    return Ok(());
                }

                info!("autobridging thread {}", thread.id);

                let discord_parent_id = if let Some(lamprey_parent_id) = thread.parent_id {
                    if let Ok(Some(parent_portal)) = self
                        .globals
                        .get_portal_by_thread_id(lamprey_parent_id)
                        .await
                    {
                        Some(parent_portal.discord_channel_id)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let name = if thread.name.is_empty() {
                    "thread".to_string()
                } else {
                    thread.name.clone()
                };
                let channel_id = discord::discord_create_channel(
                    self.globals.clone(),
                    discord_guild_id,
                    name.clone(),
                    thread.ty,
                    discord_parent_id,
                )
                .await?;

                let webhook_url = if thread.ty != common::v1::types::ThreadType::Category {
                    let webhook = discord::discord_create_webhook(
                        self.globals.clone(),
                        channel_id,
                        "bridge".to_string(),
                    )
                    .await?;
                    webhook
                        .url()
                        .map_err(|_| anyhow!("created webhook has no url"))?
                } else {
                    "".to_string()
                };

                let portal = PortalConfig {
                    lamprey_thread_id: thread.id,
                    lamprey_room_id: thread
                        .room_id
                        .ok_or_else(|| anyhow!("lamprey thread {} has no room id", thread.id))?,
                    discord_guild_id,
                    discord_channel_id: channel_id,
                    discord_thread_id: None,
                    discord_webhook: webhook_url,
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
                channel_type,
                parent_id,
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

                let thread_type = if channel_type == serenity::all::ChannelType::Category {
                    common::v1::types::ThreadType::Category
                } else {
                    common::v1::types::ThreadType::Chat
                };

                let lamprey_parent_id = if let Some(discord_parent_id) = parent_id {
                    if let Ok(Some(parent_portal)) = self
                        .globals
                        .get_portal_by_discord_channel(discord_parent_id)
                        .await
                    {
                        Some(parent_portal.lamprey_thread_id)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let thread = ly
                    .create_thread(
                        realm_config.lamprey_room_id,
                        thread_name.clone(),
                        None,
                        thread_type,
                        lamprey_parent_id,
                    )
                    .await?;

                let webhook_url = if channel_type != serenity::all::ChannelType::Category {
                    let webhook = discord::discord_create_webhook(
                        self.globals.clone(),
                        channel_id,
                        "bridge".to_string(),
                    )
                    .await?;
                    webhook
                        .url()
                        .map_err(|_| anyhow!("created webhook has no url"))?
                } else {
                    "".to_string()
                };

                let portal_config = PortalConfig {
                    lamprey_thread_id: thread.id,
                    lamprey_room_id: realm_config.lamprey_room_id,
                    discord_guild_id: guild_id,
                    discord_channel_id: channel_id,
                    discord_thread_id: None,
                    discord_webhook: webhook_url,
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
