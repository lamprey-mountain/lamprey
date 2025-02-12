use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::{mapref::one::RefMut, DashMap};
use serenity::{
    all::{
        EditWebhookMessage, EventHandler, ExecuteWebhook, GatewayIntents, Guild, Http,
        MessagePagination, Ready, Webhook,
    },
    model::prelude::{ChannelId, GuildId, Message, MessageId, MessageUpdateEvent},
    prelude::*,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use crate::{
    common::{Globals, GlobalsTrait},
    portal::{Portal, PortalMessage},
};

struct GlobalsKey;

struct Handler;

impl TypeMapKey for GlobalsKey {
    type Value = Arc<Globals>;
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("discord ready {}", ready.user.name);
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        info!("discord guild create");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        let chans = guild.channels.values().chain(&guild.threads);
        for ch in chans {
            let Some(config) = globals.config.portal_by_discord_id(ch.id) else {
                continue;
            };
            let portal = globals
                .portals
                .entry(config.my_thread_id)
                .or_insert_with(|| Portal::summon(globals.clone(), config.to_owned()));
            let last_id = globals
                .last_ids
                .iter()
                .find(|i| i.discord_channel_id == ch.id)
                .map(|i| i.discord_id);
            let Some(last_id) = last_id else {
                continue;
            };
            let mut p = MessagePagination::After(last_id);
            loop {
                let msgs = ctx
                    .http()
                    .get_messages(ch.id, Some(p), Some(100))
                    .await
                    .unwrap();
                if msgs.is_empty() {
                    break;
                }
                info!("discord backfill {} messages", msgs.len());
                let last_id = msgs.first().unwrap().id;
                for message in msgs.into_iter().rev() {
                    let _ = portal.send(PortalMessage::DiscordMessageCreate { message });
                }
                p = MessagePagination::After(last_id);
            }
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        info!("discord message create");
        let mut ctx_data = ctx.data.write().await;
        let globals = ctx_data.get_mut::<GlobalsKey>().unwrap();
        globals.portal_send_dc(
            message
                .thread
                .as_ref()
                .map(|t| t.id)
                .unwrap_or(message.channel_id),
            PortalMessage::DiscordMessageCreate { message },
        );
    }

    async fn message_update(
        &self,
        ctx: Context,
        _old_if_available: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        info!("discord message update");
        let mut ctx_data = ctx.data.write().await;
        let globals = ctx_data.get_mut::<GlobalsKey>().unwrap();
        globals.portal_send_dc(
            event.channel_id,
            PortalMessage::DiscordMessageUpdate { update: event },
        );
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete");
        let mut ctx_data = ctx.data.write().await;
        let globals = ctx_data.get_mut::<GlobalsKey>().unwrap();
        globals.portal_send_dc(
            channel_id,
            PortalMessage::DiscordMessageDelete {
                message_id: deleted_message_id,
            },
        );
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete bulk");
        let mut ctx_data = ctx.data.write().await;
        let globals = ctx_data.get_mut::<GlobalsKey>().unwrap();
        for message_id in multiple_deleted_messages_ids {
            globals.portal_send_dc(
                channel_id,
                PortalMessage::DiscordMessageDelete { message_id },
            );
        }
    }

    // async fn typing_start(&self, ctx: Context, event: TypingStartEvent) {}
}

/// discord actor
pub struct Discord {
    globals: Arc<Globals>,
    recv: mpsc::Receiver<DiscordMessage>,
    hooks: DashMap<String, Webhook>,
}

/// discord actor message
pub enum DiscordMessage {
    WebhookExecute {
        url: String,
        payload: ExecuteWebhook,
        response: oneshot::Sender<Message>,
    },
    WebhookMessageEdit {
        url: String,
        message_id: MessageId,
        payload: EditWebhookMessage,
        response: oneshot::Sender<Message>,
    },
    WebhookMessageDelete {
        url: String,
        message_id: MessageId,
        thread_id: Option<ChannelId>,
        response: oneshot::Sender<()>,
    },
    MessageGet {
        message_id: MessageId,
        channel_id: ChannelId,
        response: oneshot::Sender<Message>,
    },
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
        let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
        let handler = Handler;
        let mut client = serenity::Client::builder(token, GatewayIntents::all())
            .event_handler(handler)
            .type_map_insert::<GlobalsKey>(self.globals.clone())
            .await?;
        let http = client.http.clone();

        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                match self.handle(msg, &http).await {
                    Ok(_) => {}
                    Err(err) => error!("{err}"),
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
                    .execute(&http, true, payload)
                    .await?
                    .expect("wait should return message");
                response.send(msg).unwrap();
            }
            DiscordMessage::WebhookMessageEdit {
                url,
                message_id,
                payload,
                response,
            } => {
                let hook = self.get_hook(url, http).await?;
                let msg = hook.edit_message(&http, message_id, payload).await?;
                response.send(msg).unwrap();
            }
            DiscordMessage::WebhookMessageDelete {
                url,
                thread_id,
                message_id,
                response,
            } => {
                let hook = self.get_hook(url, http).await?;
                hook.delete_message(&http, thread_id, message_id).await?;
                response.send(()).unwrap();
            }
            DiscordMessage::MessageGet {
                message_id,
                channel_id,
                response,
            } => {
                let message = http.get_message(channel_id, message_id).await?;
                response.send(message).unwrap();
            }
        }
        Ok(())
    }

    async fn get_hook(&mut self, url: String, http: &Http) -> Result<RefMut<String, Webhook>> {
        let hook = match self.hooks.entry(url.clone()) {
            dashmap::Entry::Occupied(hook) => hook.into_ref(),
            dashmap::Entry::Vacant(vacant) => vacant.insert(Webhook::from_url(&http, &url).await?),
        };
        Ok(hook)
    }
}
