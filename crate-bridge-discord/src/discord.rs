use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use common::v1::types::{RoomId, ThreadId};
use dashmap::{mapref::one::RefMut, DashMap};
use serenity::{
    all::{
        parse_webhook, ChannelType, CreateChannel, CreateWebhook, EditWebhookMessage, EventHandler,
        ExecuteWebhook, GatewayIntents, Guild, GuildChannel, Http, MessagePagination, Permissions,
        Ready, Webhook,
    },
    model::prelude::{
        ChannelId, GuildId, Message, MessageId, MessageUpdateEvent, Reaction, TypingStartEvent,
    },
    prelude::*,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use crate::{
    common::{BridgeMessage, Globals, GlobalsTrait},
    data::{Data, PortalConfig},
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
            if globals
                .get_portal_by_discord_channel(ch.id)
                .await
                .unwrap()
                .is_some()
            {
                let config = globals
                    .get_portal_by_discord_channel(ch.id)
                    .await
                    .unwrap()
                    .unwrap();
                let portal = globals
                    .portals
                    .entry(config.lamprey_thread_id)
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
            } else {
                if ch.kind != ChannelType::Text && ch.kind != ChannelType::News {
                    continue;
                }

                let Some(_autobridge_config) = globals
                    .config
                    .autobridge
                    .iter()
                    .find(|c| c.discord_guild_id == guild.id)
                else {
                    continue;
                };

                if let Err(e) = globals
                    .bridge_chan
                    .send(BridgeMessage::DiscordChannelCreate {
                        guild_id: guild.id,
                        channel_id: ch.id,
                        channel_name: ch.name.clone(),
                    })
                    .await
                {
                    error!("failed to send discord channel create message: {e}");
                }
            }
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        info!("discord message create");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        // ignore bridged messages
        if let Some(w) = message.webhook_id {
            if let Ok(Some(h)) = globals
                .get_portal_by_discord_channel(message.channel_id)
                .await
            {
                if let Ok(webhook) = Webhook::from_url(&ctx.http, &h.discord_webhook).await {
                    if webhook.id == w {
                        return;
                    }
                }
            }
        }

        if message.content.starts_with("!disco ") {
            let args: Vec<&str> = message.content.split_whitespace().skip(1).collect();
            let guild_id = if let Some(id) = message.guild_id {
                id
            } else {
                let _ = message
                    .reply_ping(&ctx.http, "error: link command used outside of a guild")
                    .await;
                return;
            };
            let guild = ctx.cache.guild(guild_id).expect("guild").to_owned();
            let perms: Permissions = message
                .member
                .as_ref()
                .map(|m| {
                    m.roles
                        .iter()
                        .flat_map(|r| guild.roles.get(r))
                        .fold(Permissions::empty(), |p, r| p | r.permissions)
                })
                .unwrap_or_default();
            match args.get(0) {
                Some(&"link") => {
                    if !perms.manage_guild() {
                        let _ = message
                            .reply_ping(&ctx.http, "missing ManageGuild permission")
                            .await;
                        return;
                    }

                    if args.len() < 3 {
                        let _ = message
                            .reply_ping(&ctx.http, "usage: !disco link <roomId> <threadId>")
                            .await;
                        return;
                    }

                    let lamprey_room_id: RoomId = match args[1].parse() {
                        Ok(id) => id,
                        Err(_) => {
                            let _ = message
                                .reply_ping(&ctx.http, "error: invalid room id")
                                .await;
                            return;
                        }
                    };

                    let lamprey_thread_id: ThreadId = match args[2].parse() {
                        Ok(id) => id,
                        Err(_) => {
                            let _ = message
                                .reply_ping(&ctx.http, "error: invalid thread id")
                                .await;
                            return;
                        }
                    };

                    let has_existing_discord = globals
                        .get_portal_by_discord_channel(message.channel_id)
                        .await
                        .is_ok_and(|p| p.is_some());
                    if has_existing_discord {
                        let _ = message
                            .reply_ping(&ctx.http, "error: this discord channel is already bridged")
                            .await;
                        return;
                    }

                    let has_existing_lamprey = globals
                        .get_portal_by_thread_id(lamprey_thread_id)
                        .await
                        .is_ok_and(|p| p.is_some());
                    if has_existing_lamprey {
                        let _ = message
                            .reply_ping(&ctx.http, "error: that lamprey thread is already bridged")
                            .await;
                        return;
                    }

                    let thread = guild
                        .threads
                        .iter()
                        .find(|t| t.id == message.channel_id)
                        .cloned();

                    let Ok(webhook) = ctx
                        .http
                        .create_webhook(
                            thread
                                .as_ref()
                                .and_then(|t| t.parent_id)
                                .unwrap_or(message.channel_id),
                            &CreateWebhook::new("bridg"),
                            Some("for bridge"),
                        )
                        .await
                    else {
                        let _ = message
                            .reply_ping(&ctx.http, "error: could not create webhook")
                            .await;
                        return;
                    };

                    let _ = globals
                        .insert_portal(PortalConfig {
                            lamprey_thread_id,
                            lamprey_room_id,
                            discord_guild_id: guild_id,
                            discord_channel_id: thread
                                .as_ref()
                                .and_then(|t| t.parent_id)
                                .unwrap_or(message.channel_id),
                            discord_thread_id: thread.map(|t| t.id),
                            discord_webhook: webhook.url().unwrap(),
                        })
                        .await;
                    let _ = message.reply_ping(&ctx.http, "linked").await;
                }
                Some(&"unlink") => {
                    if !perms.manage_guild() {
                        let _ = message
                            .reply_ping(&ctx.http, "missing ManageGuild permission")
                            .await;
                        return;
                    }

                    if let Ok(Some(portal)) = globals
                        .get_portal_by_discord_channel(message.channel_id)
                        .await
                    {
                        if let Ok(w) = Webhook::from_url(&ctx.http, &portal.discord_webhook).await {
                            if w.delete(&ctx.http).await.is_err() {
                                let _ = message
                                    .reply_ping(&ctx.http, "warning: could not delete webhook")
                                    .await;
                            }
                        }

                        match globals.delete_portal(portal.lamprey_thread_id).await {
                            Ok(_) => {
                                let _ = message.reply_ping(&ctx.http, "done").await;
                            }
                            Err(err) => {
                                error!("failed to unlink: {err:?}");
                                let _ = message
                                    .reply_ping(&ctx.http, "failed to unlink, see logs for info")
                                    .await;
                            }
                        }
                    } else {
                        let _ = message
                            .reply_ping(&ctx.http, "this channel isnt bridged")
                            .await;
                    }
                }
                _ => {
                    let _ = message
                        .reply_ping(&ctx.http, "usage:\n!disco link [roomId] [threadId] -- link this channel to a lamprey thread\n!disco unlink -- unlink this channel\n!disco help -- print this text")
                        .await;
                }
            }
        }

        globals
            .portal_send_dc(
                message
                    .thread
                    .as_ref()
                    .map(|t| t.id)
                    .unwrap_or(message.channel_id),
                PortalMessage::DiscordMessageCreate { message },
            )
            .await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        _old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        info!("discord message update");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        // ignore bridged messages
        if let Some(w) = new.and_then(|m| m.webhook_id) {
            if let Ok(Some(h)) = globals
                .get_portal_by_discord_channel(event.channel_id)
                .await
            {
                let msg_wh_id = parse_webhook(&h.discord_webhook.parse().unwrap())
                    .unwrap()
                    .0;
                if msg_wh_id == w {
                    return;
                }
            }
        }

        globals
            .portal_send_dc(
                event.channel_id,
                PortalMessage::DiscordMessageUpdate { update: event },
            )
            .await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                channel_id,
                PortalMessage::DiscordMessageDelete {
                    message_id: deleted_message_id,
                },
            )
            .await;
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete bulk");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        for message_id in multiple_deleted_messages_ids {
            globals
                .portal_send_dc(
                    channel_id,
                    PortalMessage::DiscordMessageDelete { message_id },
                )
                .await;
        }
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        info!("discord reaction add");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                add_reaction.channel_id,
                PortalMessage::DiscordReactionAdd { add_reaction },
            )
            .await;
    }

    async fn reaction_remove(&self, ctx: Context, removed_reaction: Reaction) {
        info!("discord reaction remove");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                removed_reaction.channel_id,
                PortalMessage::DiscordReactionRemove { removed_reaction },
            )
            .await;
    }

    async fn typing_start(&self, ctx: Context, event: TypingStartEvent) {
        info!("discord typing start");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                event.channel_id,
                PortalMessage::DiscordTyping {
                    user_id: event.user_id,
                },
            )
            .await;
    }

    async fn channel_create(&self, ctx: Context, channel: GuildChannel) {
        info!("discord channel create");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        if channel.kind != ChannelType::Text && channel.kind != ChannelType::News {
            return;
        }

        let guild_id = channel.guild_id;

        if globals
            .get_portal_by_discord_channel(channel.id)
            .await
            .unwrap()
            .is_some()
        {
            return;
        }

        if let Err(e) = globals
            .bridge_chan
            .send(BridgeMessage::DiscordChannelCreate {
                guild_id,
                channel_id: channel.id,
                channel_name: channel.name.clone(),
            })
            .await
        {
            error!("failed to send discord channel create message: {e}");
        }
    }
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
    ChannelCreate {
        guild_id: GuildId,
        name: String,
        response: oneshot::Sender<ChannelId>,
    },
    WebhookCreate {
        channel_id: ChannelId,
        name: String,
        response: oneshot::Sender<Webhook>,
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
        let token = self.globals.config.discord_token.clone();
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
                    .execute(http, true, payload)
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
                let msg = hook.edit_message(http, message_id, payload).await?;
                response.send(msg).unwrap();
            }
            DiscordMessage::WebhookMessageDelete {
                url,
                thread_id,
                message_id,
                response,
            } => {
                let hook = self.get_hook(url, http).await?;
                hook.delete_message(http, thread_id, message_id).await?;
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
            DiscordMessage::ChannelCreate {
                guild_id,
                name,
                response,
            } => {
                let channel = guild_id
                    .create_channel(http, CreateChannel::new(name))
                    .await?;
                response.send(channel.id).unwrap();
            }
            DiscordMessage::WebhookCreate {
                channel_id,
                name,
                response,
            } => {
                let hook = channel_id
                    .create_webhook(http, CreateWebhook::new(name))
                    .await?;
                response.send(hook).unwrap();
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

pub async fn discord_create_channel(
    globals: Arc<Globals>,
    guild_id: GuildId,
    name: String,
) -> Result<serenity::all::ChannelId> {
    let (send, recv) = oneshot::channel();
    globals
        .dc_chan
        .send(DiscordMessage::ChannelCreate {
            guild_id,
            name,
            response: send,
        })
        .await?;
    Ok(recv.await?)
}

pub async fn discord_create_webhook(
    globals: Arc<Globals>,
    channel_id: serenity::all::ChannelId,
    name: String,
) -> Result<Webhook> {
    let (send, recv) = oneshot::channel();
    globals
        .dc_chan
        .send(DiscordMessage::WebhookCreate {
            channel_id,
            name,
            response: send,
        })
        .await?;
    Ok(recv.await?)
}
