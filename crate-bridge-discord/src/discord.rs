use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use common::v1::types::{RoomId, ThreadId};
use dashmap::{mapref::one::RefMut, DashMap};
use serenity::{
    all::{
        parse_webhook, ChannelType, CommandDataOptionValue, CommandInteraction, CommandOptionType,
        CreateChannel, CreateCommand, CreateCommandOption, CreateInteractionResponseMessage,
        CreateWebhook, EditWebhookMessage, EventHandler, ExecuteWebhook, GatewayIntents, Guild,
        GuildChannel, Http, Interaction, InteractionContext, InteractionResponseFlags,
        MessagePagination, Permissions, Ready, Webhook,
    },
    model::prelude::{
        ChannelId, GuildId, Message, MessageId, MessageUpdateEvent, Reaction, TypingStartEvent,
    },
    prelude::*,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use crate::{
    common::{BridgeMessage, Globals, GlobalsTrait, PortalConfig},
    data::Data,
    portal::{Portal, PortalMessage},
};

struct GlobalsKey;

struct Handler;

impl TypeMapKey for GlobalsKey {
    type Value = Arc<Globals>;
}

async fn send_ephemeral_reply(ctx: &Context, command: &CommandInteraction, content: &str) {
    let builder = CreateInteractionResponseMessage::new()
        .content(content.to_string())
        .flags(InteractionResponseFlags::EPHEMERAL);
    let response = serenity::all::CreateInteractionResponse::Message(builder);
    if let Err(err) = command.create_response(&ctx.http, response).await {
        error!("failed to respond to interaction: {err:?}");
    }
}

fn get_commands() -> Vec<CreateCommand> {
    let ping = CreateCommand::new("ping")
        .description("healthcheck for the bridge")
        .default_member_permissions(Permissions::from_bits_truncate(536870944));

    let link = CreateCommand::new("link")
            .description("link something to lamprey")
            .default_member_permissions(Permissions::from_bits_truncate(536870944))
            .contexts(vec![InteractionContext::Guild])
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "guild", "link this guild")
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "room_id",
                            "the uuid of the room to link to",
                        )
                        .required(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "backfill",
                            "whether to clone the full history of every channel",
                        )
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "continuous",
                            "whether to create new portals as channels/threads are created (this is bidirectional)",
                        )
                    ),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "channel", "link this channel")
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "room_id",
                            "the uuid of the room to link to",
                        )
                        .required(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "thread_id",
                            "the uuid of the thread to link to",
                        )
                        .required(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "backfill",
                            "whether to clone the full history of this channel",
                        )
                    )
            );

    let unlink = CreateCommand::new("unlink")
        .description("unlink something from lamprey")
        .default_member_permissions(Permissions::from_bits_truncate(536870944))
        .contexts(vec![InteractionContext::Guild])
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "guild",
            "unlink this guild",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "channel",
            "unlink this channel",
        ));

    vec![ping, link, unlink]
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("discord ready {}", ready.user.name);

        if let Err(err) = ctx.http.create_global_commands(&get_commands()).await {
            error!("error while registering commands: {err:?}")
        }
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

                let Ok(realms) = globals.get_realms().await else {
                    continue;
                };

                let Some(realm_config) = realms.iter().find(|c| c.discord_guild_id == guild.id)
                else {
                    continue;
                };

                if !realm_config.continuous {
                    continue;
                }

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

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        info!("interaction create {interaction:?}");

        let Some(command) = interaction.into_command() else {
            return;
        };

        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap().clone();

        match command.data.name.as_str() {
            "ping" => {
                send_ephemeral_reply(&ctx, &command, "pong!").await;
            }
            "link" => {
                let guild_id = command.guild_id.unwrap();
                let channel_id = command.channel_id;

                let subcommand = &command.data.options[0];
                match subcommand.name.as_str() {
                    "channel" => {
                        if let CommandDataOptionValue::SubCommand(options) = &subcommand.value {
                            let mut room_id_str = None;
                            let mut thread_id_str = None;
                            for opt in options {
                                match opt.name.as_str() {
                                    "room_id" => {
                                        room_id_str = opt.value.as_str().to_owned();
                                    }
                                    "thread_id" => {
                                        thread_id_str = opt.value.as_str().to_owned();
                                    }
                                    _ => {}
                                }
                            }

                            let (Some(room_id_str), Some(thread_id_str)) =
                                (room_id_str, thread_id_str)
                            else {
                                send_ephemeral_reply(
                                    &ctx,
                                    &command,
                                    "error: missing required options",
                                )
                                .await;
                                return;
                            };

                            let lamprey_room_id: RoomId = match room_id_str.parse() {
                                Ok(id) => id,
                                Err(_) => {
                                    send_ephemeral_reply(&ctx, &command, "error: invalid room id")
                                        .await;
                                    return;
                                }
                            };

                            let lamprey_thread_id: ThreadId = match thread_id_str.parse() {
                                Ok(id) => id,
                                Err(_) => {
                                    send_ephemeral_reply(
                                        &ctx,
                                        &command,
                                        "error: invalid thread id",
                                    )
                                    .await;
                                    return;
                                }
                            };

                            let has_existing_discord = globals
                                .get_portal_by_discord_channel(channel_id)
                                .await
                                .is_ok_and(|p| p.is_some());
                            if has_existing_discord {
                                send_ephemeral_reply(
                                    &ctx,
                                    &command,
                                    "error: this discord channel is already bridged",
                                )
                                .await;
                                return;
                            }

                            let has_existing_lamprey = globals
                                .get_portal_by_thread_id(lamprey_thread_id)
                                .await
                                .is_ok_and(|p| p.is_some());
                            if has_existing_lamprey {
                                send_ephemeral_reply(
                                    &ctx,
                                    &command,
                                    "error: that lamprey thread is already bridged",
                                )
                                .await;
                                return;
                            }

                            let guild = ctx.cache.guild(guild_id).expect("guild").to_owned();
                            let thread = guild.threads.iter().find(|t| t.id == channel_id).cloned();

                            let Ok(webhook) = ctx
                                .http
                                .create_webhook(
                                    thread
                                        .as_ref()
                                        .and_then(|t| t.parent_id)
                                        .unwrap_or(channel_id),
                                    &CreateWebhook::new("bridg"),
                                    Some("for bridge"),
                                )
                                .await
                            else {
                                send_ephemeral_reply(
                                    &ctx,
                                    &command,
                                    "error: could not create webhook",
                                )
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
                                        .unwrap_or(channel_id),
                                    discord_thread_id: thread.map(|t| t.id),
                                    discord_webhook: webhook.url().unwrap(),
                                })
                                .await;
                            send_ephemeral_reply(&ctx, &command, "linked").await;
                        }
                    }
                    "guild" => {
                        if let CommandDataOptionValue::SubCommand(options) = &subcommand.value {
                            let mut room_id_str = None;
                            let mut continuous = None;
                            for opt in options {
                                match opt.name.as_str() {
                                    "room_id" => {
                                        room_id_str = opt.value.as_str().to_owned();
                                    }
                                    "continuous" => {
                                        continuous = opt.value.as_bool();
                                    }
                                    _ => {}
                                }
                            }

                            let Some(room_id_str) = room_id_str else {
                                send_ephemeral_reply(&ctx, &command, "error: missing room_id")
                                    .await;
                                return;
                            };

                            let lamprey_room_id: RoomId = match room_id_str.parse() {
                                Ok(id) => id,
                                Err(_) => {
                                    send_ephemeral_reply(&ctx, &command, "error: invalid room id")
                                        .await;
                                    return;
                                }
                            };

                            let realm_config = crate::common::RealmConfig {
                                lamprey_room_id,
                                discord_guild_id: guild_id,
                                continuous: continuous.unwrap_or(false),
                            };

                            if let Err(e) = globals.insert_realm(realm_config).await {
                                error!("failed to insert realm: {e}");
                                send_ephemeral_reply(&ctx, &command, "error: failed to link guild")
                                    .await;
                                return;
                            }

                            send_ephemeral_reply(&ctx, &command, "guild linked").await;
                        }
                    }
                    _ => {}
                }
            }
            "unlink" => {
                let guild_id = command.guild_id.unwrap();
                let channel_id = command.channel_id;

                let subcommand = &command.data.options[0];
                match subcommand.name.as_str() {
                    "channel" => {
                        if let Ok(Some(portal)) =
                            globals.get_portal_by_discord_channel(channel_id).await
                        {
                            if let Ok(w) =
                                Webhook::from_url(&ctx.http, &portal.discord_webhook).await
                            {
                                if w.delete(&ctx.http).await.is_err() {
                                    send_ephemeral_reply(
                                        &ctx,
                                        &command,
                                        "warning: could not delete webhook",
                                    )
                                    .await;
                                }
                            }

                            match globals.delete_portal(portal.lamprey_thread_id).await {
                                Ok(_) => {
                                    send_ephemeral_reply(&ctx, &command, "done").await;
                                }
                                Err(err) => {
                                    error!("failed to unlink: {err:?}");
                                    send_ephemeral_reply(
                                        &ctx,
                                        &command,
                                        "failed to unlink, see logs for info",
                                    )
                                    .await;
                                }
                            }
                        } else {
                            send_ephemeral_reply(&ctx, &command, "this channel isnt bridged").await;
                        }
                    }
                    "guild" => {
                        let realms = match globals.get_realms().await {
                            Ok(r) => r,
                            Err(e) => {
                                error!("failed to get realms: {e}");
                                send_ephemeral_reply(&ctx, &command, "error: failed to get realms")
                                    .await;
                                return;
                            }
                        };

                        let Some(realm) = realms.iter().find(|r| r.discord_guild_id == guild_id)
                        else {
                            send_ephemeral_reply(&ctx, &command, "error: this guild is not linked")
                                .await;
                            return;
                        };

                        if let Err(e) = globals.delete_realm(realm.lamprey_room_id).await {
                            error!("failed to delete realm: {e}");
                            send_ephemeral_reply(&ctx, &command, "error: failed to unlink guild")
                                .await;
                            return;
                        }

                        send_ephemeral_reply(&ctx, &command, "guild unlinked").await;
                    }
                    _ => {}
                }
            }
            _ => {}
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

    async fn get_hook(&mut self, url: String, http: &Http) -> Result<RefMut<'_, String, Webhook>> {
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
