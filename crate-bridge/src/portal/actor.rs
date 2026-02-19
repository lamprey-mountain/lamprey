use std::fmt::Debug;
use std::sync::Arc;

use crate::bridge_common::{Globals, PortalConfig};

use anyhow::Result;
use serenity::all::ChannelId as DcChannelId;
use tokio::sync::mpsc;
use tracing::error;

use crate::portal::messages::PortalMessage;

pub struct Portal {
    pub globals: Arc<Globals>,
    recv: mpsc::UnboundedReceiver<PortalMessage>,
    pub config: PortalConfig,
}

impl Debug for Portal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Portal ({:?})", self.config)
    }
}

impl Portal {
    pub fn summon(
        globals: Arc<Globals>,
        config: PortalConfig,
    ) -> mpsc::UnboundedSender<PortalMessage> {
        let (send, recv) = mpsc::unbounded_channel();
        let portal = Self {
            globals,
            recv,
            config,
        };
        tokio::spawn(portal.activate());
        send
    }

    pub fn channel_id(&self) -> DcChannelId {
        self.config.discord_channel_id
    }

    pub fn thread_id(&self) -> common::v1::types::ChannelId {
        self.config.lamprey_thread_id
    }

    pub fn room_id(&self) -> common::v1::types::RoomId {
        self.config.lamprey_room_id
    }

    async fn activate(mut self) {
        while let Some(msg) = self.recv.recv().await {
            if let Err(err) = self.handle(msg).await {
                error!(portal = ?self.config, "error handling portal message: {err:?}");
            }
        }
        error!(portal = ?self.config, "portal channel closed, shutting down");
    }

    #[tracing::instrument(
        skip(self),
        fields(
            lamprey_thread_id = %self.config.lamprey_thread_id,
            discord_channel_id = %self.config.discord_channel_id,
        )
    )]
    async fn handle(&mut self, msg: PortalMessage) -> Result<()> {
        match msg {
            PortalMessage::LampreyMessageCreate { message } => {
                self.handle_lamprey_message_create(message).await?;
            }
            PortalMessage::LampreyMessageUpdate { message } => {
                self.handle_lamprey_message_create(message).await?;
            }
            PortalMessage::LampreyMessageDelete { message_id } => {
                self.handle_lamprey_message_delete(message_id).await?;
            }
            PortalMessage::DiscordMessageCreate { message } => {
                self.handle_discord_message_create(message).await?;
            }
            PortalMessage::DiscordMessageUpdate {
                update,
                new_message,
            } => {
                self.handle_discord_message_update(update, new_message)
                    .await?;
            }
            PortalMessage::DiscordMessageDelete { message_id } => {
                self.handle_discord_message_delete(message_id).await?;
            }
            PortalMessage::DiscordReactionAdd { add_reaction } => {
                self.handle_discord_reaction_add(add_reaction).await?;
            }
            PortalMessage::DiscordReactionRemove { removed_reaction } => {
                self.handle_discord_reaction_remove(removed_reaction)
                    .await?;
            }
            PortalMessage::DiscordTyping { user_id } => {
                self.handle_discord_typing(user_id).await?;
            }
        }
        Ok(())
    }
}
