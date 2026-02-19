use std::sync::Arc;

use anyhow::Result;
use dashmap::{mapref::one::RefMut, DashMap};
use serenity::all::{ChannelType, CreateChannel, CreateWebhook, GatewayIntents, Http, Webhook};
use tokio::sync::mpsc;
use tracing::error;

use crate::bridge_common::Globals;
use crate::discord::events::Handler;
use crate::discord::messages::DiscordMessage;

/// discord actor
pub struct Discord {
    globals: Arc<Globals>,
    recv: mpsc::Receiver<DiscordMessage>,
    hooks: DashMap<String, Webhook>,
}

impl Discord {
    pub fn new(globals: Arc<Globals>, recv: mpsc::Receiver<DiscordMessage>) -> Discord {
        Discord {
            globals,
            recv,
            hooks: DashMap::new(),
        }
    }

    pub async fn connect(mut self) -> Result<()> {
        let token = self.globals.config.discord_token.clone();
        let handler = Handler;
        let mut client = serenity::Client::builder(token, GatewayIntents::all())
            .event_handler(handler)
            .type_map_insert::<crate::discord::events::GlobalsKey>(self.globals.clone())
            .await?;
        let http = client.http.clone();

        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                match self.handle(msg, &http).await {
                    Ok(_) => {}
                    Err(err) => error!("error handling event: {err}"),
                };
            }
        });

        client.start().await?;

        Ok(())
    }

    async fn handle(&mut self, msg: DiscordMessage, http: &Http) -> Result<()> {
        match msg {
            DiscordMessage::WebhookExecute {
                url,
                payload,
                response,
            } => {
                let hook = self.get_hook(url, http).await?;

                let msg = hook
                    .execute(http, true, payload)
                    .await?
                    .expect("wait should return message");

                if response.send(msg).is_err() {
                    error!("failed to send response to portal: receiver dropped");
                }
            }

            DiscordMessage::WebhookMessageEdit {
                url,
                message_id,
                payload,
                response,
            } => {
                let hook = self.get_hook(url, http).await?;

                let msg = hook.edit_message(http, message_id, payload).await?;

                if response.send(msg).is_err() {
                    error!("failed to send response to portal: receiver dropped");
                }
            }
            DiscordMessage::WebhookMessageDelete {
                url,
                thread_id,
                message_id,
                response,
            } => {
                let hook = self.get_hook(url, http).await?;

                hook.delete_message(http, thread_id, message_id).await?;

                if response.send(()).is_err() {
                    error!("failed to send response to portal: receiver dropped");
                }
            }
            DiscordMessage::MessageGet {
                message_id,
                channel_id,
                response,
            } => {
                let message = http.get_message(channel_id, message_id).await?;

                if response.send(message).is_err() {
                    error!("failed to send response to portal: receiver dropped");
                }
            }
            DiscordMessage::ChannelCreate {
                guild_id,
                name,
                ty,
                parent_id,
                response,
            } => {
                let mut channel = CreateChannel::new(name).kind(match ty {
                    common::v1::types::ChannelType::Category => ChannelType::Category,
                    _ => ChannelType::Text,
                });
                if let Some(parent_id) = parent_id {
                    channel = channel.category(parent_id);
                }
                let channel = guild_id.create_channel(http, channel).await?;
                self.globals
                    .recently_created_discord_channels
                    .insert(channel.id, ());
                if response.send(channel.id).is_err() {
                    error!("failed to send response to portal: receiver dropped");
                }
            }
            DiscordMessage::WebhookCreate {
                channel_id,
                name,
                response,
            } => {
                let hook = channel_id
                    .create_webhook(http, CreateWebhook::new(name))
                    .await?;

                if response.send(hook).is_err() {
                    error!("failed to send response to portal: receiver dropped");
                }
            }
        }
        Ok(())
    }

    async fn get_hook(&mut self, url: String, http: &Http) -> Result<RefMut<'_, String, Webhook>> {
        let hook = match self.hooks.entry(url.clone()) {
            dashmap::Entry::Occupied(hook) => hook.into_ref(),
            dashmap::Entry::Vacant(vacant) => vacant.insert(Webhook::from_url(&http, &url).await?),
        };
        Ok(hook)
    }
}
