use std::sync::Arc;

use anyhow::Result;
use dashmap::{mapref::one::RefMut, DashMap};
use serenity::all::{ChannelId as DcChannelId, GuildId as DcGuildId, MessageId as DcMessageId};
use serenity::all::{ChannelType, CreateChannel, CreateWebhook, GatewayIntents, Http, Webhook};

use crate::bridge_common::Globals;
use crate::discord::events::Handler;

/// Discord actor - manages Discord API interactions
///
/// Note: This actor is NOT spawned via kameo because serenity runs its own event loop.
/// The serenity client consumes `self` in `connect()`, so we can't use kameo's spawn.
/// Instead, we store the Discord instance in an Arc<RwLock> and process messages directly.
/// The Message trait implementation provides type-safe request/response handling.
pub struct Discord {
    pub globals: Arc<Globals>,
    pub hooks: DashMap<String, Webhook>,
}

/// Discord actor messages - fully using Kameo ask pattern
#[derive(Debug)]
pub enum DiscordMessage {
    WebhookExecute {
        url: String,
        payload: serenity::all::ExecuteWebhook,
    },
    WebhookMessageEdit {
        url: String,
        message_id: DcMessageId,
        payload: serenity::all::EditWebhookMessage,
    },
    WebhookMessageDelete {
        url: String,
        thread_id: Option<DcChannelId>,
        message_id: DcMessageId,
    },
    MessageGet {
        message_id: DcMessageId,
        channel_id: DcChannelId,
    },
    ChannelCreate {
        guild_id: DcGuildId,
        name: String,
        ty: common::v1::types::ChannelType,
        parent_id: Option<DcChannelId>,
    },
    WebhookCreate {
        channel_id: DcChannelId,
        name: String,
    },
}

/// Response types for DiscordMessage requests
pub enum DiscordResponse {
    Unit,
    Message(serenity::all::Message),
    ChannelId(DcChannelId),
    Webhook(Webhook),
}

impl Discord {
    pub fn new(globals: Arc<Globals>) -> Discord {
        Discord {
            globals,
            hooks: DashMap::new(),
        }
    }

    /// Start the Discord actor - runs serenity's event loop
    /// This is a long-running future that should be spawned
    pub async fn connect(self) -> Result<()> {
        let token = self.globals.config.discord_token.clone();
        let handler = Handler;
        let mut client = serenity::Client::builder(token, GatewayIntents::all())
            .event_handler(handler)
            .type_map_insert::<crate::discord::events::GlobalsKey>(self.globals.clone())
            .await?;

        client.start().await?;

        Ok(())
    }

    /// Handle a DiscordMessage directly (used by serenity event handlers)
    pub async fn handle_message(&mut self, msg: DiscordMessage) -> Result<DiscordResponse> {
        match msg {
            DiscordMessage::WebhookExecute { url, payload } => {
                let http = Http::new(&self.globals.config.discord_token);
                let hook = self.get_hook(url, &http).await?;

                let msg = hook
                    .execute(&http, true, payload)
                    .await?
                    .expect("wait should return message");

                Ok(DiscordResponse::Message(msg))
            }

            DiscordMessage::WebhookMessageEdit {
                url,
                message_id,
                payload,
            } => {
                let http = Http::new(&self.globals.config.discord_token);
                let hook = self.get_hook(url, &http).await?;

                let msg = hook.edit_message(&http, message_id, payload).await?;
                Ok(DiscordResponse::Message(msg))
            }
            DiscordMessage::WebhookMessageDelete {
                url,
                thread_id,
                message_id,
            } => {
                let http = Http::new(&self.globals.config.discord_token);
                let hook = self.get_hook(url, &http).await?;

                hook.delete_message(&http, thread_id, message_id).await?;
                Ok(DiscordResponse::Unit)
            }
            DiscordMessage::MessageGet {
                message_id,
                channel_id,
            } => {
                let http = Http::new(&self.globals.config.discord_token);
                let message = http.get_message(channel_id, message_id).await?;
                Ok(DiscordResponse::Message(message))
            }
            DiscordMessage::ChannelCreate {
                guild_id,
                name,
                ty,
                parent_id,
            } => {
                let http = Http::new(&self.globals.config.discord_token);
                let mut channel = CreateChannel::new(name).kind(match ty {
                    common::v1::types::ChannelType::Category => ChannelType::Category,
                    _ => ChannelType::Text,
                });
                if let Some(parent_id) = parent_id {
                    channel = channel.category(parent_id);
                }
                let channel = guild_id.create_channel(&http, channel).await?;
                self.globals
                    .recently_created_discord_channels
                    .insert(channel.id, ());
                Ok(DiscordResponse::ChannelId(channel.id))
            }
            DiscordMessage::WebhookCreate { channel_id, name } => {
                let http = Http::new(&self.globals.config.discord_token);
                let hook = channel_id
                    .create_webhook(&http, CreateWebhook::new(name))
                    .await?;
                Ok(DiscordResponse::Webhook(hook))
            }
        }
    }

    async fn get_hook(&mut self, url: String, http: &Http) -> Result<RefMut<'_, String, Webhook>> {
        let hook = match self.hooks.entry(url.clone()) {
            dashmap::Entry::Occupied(hook) => hook.into_ref(),
            dashmap::Entry::Vacant(vacant) => vacant.insert(Webhook::from_url(http, &url).await?),
        };
        Ok(hook)
    }
}
