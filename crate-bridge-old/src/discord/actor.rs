use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use serenity::all::{ChannelId as DcChannelId, GuildId as DcGuildId, MessageId as DcMessageId};
use serenity::all::{ChannelType, CreateChannel, CreateWebhook, GatewayIntents, Http, Webhook};

use crate::bridge_common::Globals;
use crate::discord::events::Handler;

/// Discord actor - manages Discord API interactions
///
/// Note: This actor is NOT spawned via kameo because serenity runs its own event loop.
/// The serenity client consumes `self` in `connect()`, so we can't use kameo's spawn.
/// Instead, we store the Discord instance in an Arc<OnceCell> and clone for connect().
#[derive(Clone)]
pub struct Discord {
    pub globals: Arc<Globals>,
    pub hooks: Arc<DashMap<String, Webhook>>,
    pub http: Arc<Http>,
}

impl Discord {
    pub fn new(globals: Arc<Globals>) -> Self {
        let http = Arc::new(Http::new(&globals.config.discord_token));
        Self {
            globals,
            hooks: Arc::new(DashMap::new()),
            http,
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

    /// Execute a webhook and return the created message
    pub async fn execute_webhook(
        &self,
        url: &str,
        payload: serenity::all::ExecuteWebhook,
    ) -> Result<serenity::all::Message> {
        let hook = self.get_hook(url).await?;
        let msg = hook
            .execute(&self.http, true, payload)
            .await?
            .expect("wait should return message");
        Ok(msg)
    }

    /// Edit a webhook message
    pub async fn edit_webhook_message(
        &self,
        url: &str,
        message_id: DcMessageId,
        payload: serenity::all::EditWebhookMessage,
    ) -> Result<serenity::all::Message> {
        let hook = self.get_hook(url).await?;
        let msg = hook.edit_message(&self.http, message_id, payload).await?;
        Ok(msg)
    }

    /// Delete a webhook message
    pub async fn delete_webhook_message(
        &self,
        url: &str,
        thread_id: Option<DcChannelId>,
        message_id: DcMessageId,
    ) -> Result<()> {
        let hook = self.get_hook(url).await?;
        hook.delete_message(&self.http, thread_id, message_id)
            .await?;
        Ok(())
    }

    /// Get a Discord message by channel and message ID
    pub async fn get_message(
        &self,
        channel_id: DcChannelId,
        message_id: DcMessageId,
    ) -> Result<serenity::all::Message> {
        let msg = self.http.get_message(channel_id, message_id).await?;
        Ok(msg)
    }

    /// Create a Discord channel
    pub async fn create_channel(
        &self,
        guild_id: DcGuildId,
        name: String,
        ty: common::v1::types::ChannelType,
        parent_id: Option<DcChannelId>,
    ) -> Result<DcChannelId> {
        let mut channel = CreateChannel::new(name).kind(match ty {
            common::v1::types::ChannelType::Category => ChannelType::Category,
            _ => ChannelType::Text,
        });
        if let Some(parent_id) = parent_id {
            channel = channel.category(parent_id);
        }
        let channel = guild_id.create_channel(&self.http, channel).await?;
        self.globals
            .recently_created_discord_channels
            .insert(channel.id, ());
        Ok(channel.id)
    }

    /// Create a webhook in a channel
    pub async fn create_webhook(&self, channel_id: DcChannelId, name: String) -> Result<Webhook> {
        let hook = channel_id
            .create_webhook(&self.http, CreateWebhook::new(name))
            .await?;
        Ok(hook)
    }

    async fn get_hook(&self, url: &str) -> Result<Webhook> {
        // First try to get existing hook
        if let Some(hook) = self.hooks.get(url) {
            return Ok(hook.clone());
        }
        // Create new hook
        let hook = Webhook::from_url(&self.http, url).await?;
        self.hooks.insert(url.to_owned(), hook.clone());
        Ok(hook)
    }
}
