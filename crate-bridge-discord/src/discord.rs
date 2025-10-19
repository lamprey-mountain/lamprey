use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use common::v1::types::{ChannelId as LyChannelId, RoomId};
use dashmap::{mapref::one::RefMut, DashMap};
use serenity::{
    all::{
        ChannelType, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateChannel,
        CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateWebhook,
        EditWebhookMessage, EventHandler, ExecuteWebhook, GatewayIntents, Guild, GuildChannel,
        GuildMemberUpdateEvent, Http, Interaction, InteractionContext, InteractionResponseFlags,
        MessagePagination, Permissions, Presence, Ready, Webhook,
    },
    model::prelude::{
        ActivityType, ChannelId, GuildId, Message, MessageId, MessageUpdateEvent, OnlineStatus,
        Reaction, TypingStartEvent,
    },
    prelude::*,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, trace, warn, Instrument};

use crate::{
    bridge::BridgeMessage,
    common::{Globals, GlobalsTrait, PortalConfig, RealmConfig, WEBHOOK_NAME},
    data::Data,
    portal::{Portal, PortalMessage},
};
use common::v1::types::user_status::{StatusPatch, StatusText, StatusTypePatch};

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

async fn backfill_channel(
    ctx: &Context,
    globals: Arc<Globals>,
    channel_id: ChannelId,
) -> Result<()> {
    let Some(config) = globals.get_portal_by_discord_channel(channel_id).await? else {
        warn!("backfill_channel: no portal for {}", channel_id);
        return Ok(());
    };

    let portal = globals
        .portals
        .entry(config.lamprey_thread_id)
        .or_insert_with(|| Portal::summon(globals.clone(), config.to_owned()));

    let mut p = MessagePagination::After(MessageId::new(1));
    loop {
        let msgs = ctx
            .http()
            .get_messages(channel_id, Some(p), Some(100))
            .await?;

        if msgs.is_empty() {
            break;
        }

        info!(
            "discord backfill {} messages for channel {}",
            msgs.len(),
            channel_id
        );

        let last_id = msgs.first().unwrap().id;
        for message in msgs.into_iter().rev() {
            if globals.get_message_dc(message.id).await?.is_some() {
                debug!("skipping already bridged message: {}", message.id);
                continue;
            }
            let _ = portal.send(PortalMessage::DiscordMessageCreate { message });
        }
        p = MessagePagination::After(last_id);
    }
    info!("finished backfill for channel {}", channel_id);
    Ok(())
}

async fn backfill_guild(
    ctx: &Context,
    globals: Arc<Globals>,
    guild_id: GuildId,
    realm_config: RealmConfig,
) -> Result<()> {
    let guild = ctx
        .cache
        .guild(guild_id)
        .ok_or(anyhow!("failed to get guild {guild_id} from cache"))?
        .to_owned();

    let mut all_channels: Vec<_> = guild.channels.values().chain(&guild.threads).collect();
    all_channels.sort_by_key(|c| c.parent_id.is_some());

    for channel in all_channels {
        if !matches!(
            channel.kind,
            ChannelType::Text
                | ChannelType::News
                | ChannelType::PublicThread
                | ChannelType::PrivateThread
                | ChannelType::NewsThread
                | ChannelType::Category
        ) {
            continue;
        }

        if globals
            .get_portal_by_discord_channel(channel.id)
            .await
            .is_ok_and(|p| p.is_some())
        {
            let ctx = ctx.clone();
            let globals = globals.clone();
            let channel_id = channel.id;
            tokio::spawn(async move {
                if let Err(e) = backfill_channel(&ctx, globals, channel_id).await {
                    error!(
                        "failed to backfill existing portal for channel {}: {}",
                        channel_id, e
                    );
                }
            });
            continue;
        }

        // create portal
        let ly = globals.lamprey_handle().await?;

        let thread_type = if channel.kind == ChannelType::Category {
            common::v1::types::ChannelType::Category
        } else {
            common::v1::types::ChannelType::Text
        };

        let lamprey_parent_id = if let Some(discord_parent_id) = channel.parent_id {
            if let Ok(Some(parent_portal)) = globals
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
                channel.name.clone(),
                None,
                thread_type,
                lamprey_parent_id,
            )
            .await?;

        let (is_thread, parent_id) = if matches!(
            channel.kind,
            ChannelType::PublicThread | ChannelType::PrivateThread | ChannelType::NewsThread
        ) {
            (true, channel.parent_id)
        } else {
            (false, Some(channel.id))
        };

        let webhook_url = if channel.kind != ChannelType::Category {
            let Some(webhook_channel_id) = parent_id else {
                info!("channel {} has no parent, skipping", channel.id);
                continue;
            };
            let webhook = discord_create_webhook(
                globals.clone(),
                webhook_channel_id,
                WEBHOOK_NAME.to_string(),
            )
            .await?;
            webhook.url().unwrap().to_string()
        } else {
            "".to_string()
        };

        let portal_config = PortalConfig {
            lamprey_thread_id: thread.id,
            lamprey_room_id: realm_config.lamprey_room_id,
            discord_guild_id: guild_id,
            discord_channel_id: parent_id.unwrap_or(channel.id),
            discord_thread_id: if is_thread { Some(channel.id) } else { None },
            discord_webhook: webhook_url,
        };

        globals.insert_portal(portal_config.clone()).await?;

        globals
            .portals
            .entry(portal_config.lamprey_thread_id)
            .or_insert_with(|| Portal::summon(globals.clone(), portal_config));

        let globals = globals.clone();
        let ctx = ctx.clone();
        let channel_id = channel.id;
        tokio::spawn(async move {
            if let Err(e) = backfill_channel(&ctx, globals, channel_id).await {
                error!(
                    "failed to backfill new portal for channel {}: {}",
                    channel_id, e
                );
            }
        });
    }

    Ok(())
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

        async {
            let chans = guild.channels.values().chain(&guild.threads);
            for ch in chans {
                async {
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

                        let last_id = globals.last_discord_ids.get(&ch.id).map(|v| *v.value());
                        let Some(last_id) = last_id else {
                            return;
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
                                if globals.get_message_dc(message.id).await.unwrap().is_some() {
                                    debug!("skipping already bridged message: {}", message.id);
                                    continue;
                                }
                                let _ =
                                    portal.send(PortalMessage::DiscordMessageCreate { message });
                            }
                            p = MessagePagination::After(last_id);
                        }
                    } else {
                        if ch.kind != ChannelType::Text && ch.kind != ChannelType::News {
                            return;
                        }

                        let Ok(realms) = globals.get_realms().await else {
                            return;
                        };

                        let Some(realm_config) =
                            realms.iter().find(|c| c.discord_guild_id == guild.id)
                        else {
                            return;
                        };

                        if !realm_config.continuous {
                            return;
                        }

                        info!("no portal exists so we'll create one");

                        if let Err(e) =
                            globals
                                .bridge_chan
                                .send(BridgeMessage::DiscordChannelCreate {
                                    guild_id: guild.id,
                                    channel_id: ch.id,
                                    channel_name: ch.name.clone(),
                                    channel_type: ch.kind,
                                    parent_id: ch.parent_id,
                                })
                        {
                            error!("failed to send discord channel create message: {e}");
                        }
                    }
                }
                .instrument(tracing::debug_span!("incremental backfill channel", ?ch.id))
                .await;
            }
        }
        .instrument(tracing::debug_span!("incremental backfill guild", ?guild.id))
        .await;
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

        let mut message_with_full_author = message.clone();
        let user_id = message.author.id;

        let cached_user = globals.discord_user_cache.get(&user_id);
        if cached_user.is_some()
            && cached_user.as_ref().unwrap().fetched_at.elapsed().as_secs() < 300
        {
            message_with_full_author.author = cached_user.unwrap().user.clone();
        } else {
            match ctx.http.get_user(user_id).await {
                Ok(user) => {
                    globals.discord_user_cache.insert(
                        user_id,
                        crate::common::UserCacheEntry {
                            user: user.clone(),
                            fetched_at: std::time::Instant::now(),
                        },
                    );
                    message_with_full_author.author = user;
                }
                Err(e) => error!("Failed to fetch full user object for {}: {}", user_id, e),
            }
        }

        globals
            .portal_send_dc(
                message
                    .thread
                    .as_ref()
                    .map(|t| t.id)
                    .unwrap_or(message.channel_id),
                PortalMessage::DiscordMessageCreate {
                    message: message_with_full_author,
                },
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
        if let Some(w) = new.as_ref().and_then(|m| m.webhook_id) {
            if let Ok(Some(h)) = globals
                .get_portal_by_discord_channel(event.channel_id)
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
                event.channel_id,
                PortalMessage::DiscordMessageUpdate {
                    update: event,
                    new_message: new,
                },
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

        if !matches!(
            channel.kind,
            ChannelType::Text | ChannelType::News | ChannelType::Category
        ) {
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
                channel_type: channel.kind,
                parent_id: channel.parent_id,
            })
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
                            let mut backfill = None;
                            for opt in options {
                                match opt.name.as_str() {
                                    "room_id" => {
                                        room_id_str = opt.value.as_str().to_owned();
                                    }
                                    "thread_id" => {
                                        thread_id_str = opt.value.as_str().to_owned();
                                    }
                                    "backfill" => {
                                        backfill = opt.value.as_bool();
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

                            let lamprey_room_id: RoomId = match dbg!(room_id_str).parse() {
                                Ok(id) => id,
                                Err(_) => {
                                    send_ephemeral_reply(&ctx, &command, "error: invalid room id")
                                        .await;
                                    return;
                                }
                            };

                            let lamprey_thread_id: LyChannelId = match thread_id_str.parse() {
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
                                    &CreateWebhook::new(WEBHOOK_NAME),
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

                            if backfill.unwrap_or(false) {
                                send_ephemeral_reply(&ctx, &command, "linked, backfilling...")
                                    .await;
                                let ctx = ctx.clone();
                                let globals = globals.clone();
                                tokio::spawn(async move {
                                    if let Err(e) =
                                        backfill_channel(&ctx, globals, channel_id).await
                                    {
                                        error!("failed to backfill channel {}: {}", channel_id, e);
                                    }
                                });
                            } else {
                                send_ephemeral_reply(&ctx, &command, "linked").await;
                            }
                        }
                    }
                    "guild" => {
                        if let CommandDataOptionValue::SubCommand(options) = &subcommand.value {
                            let mut room_id_str = None;
                            let mut continuous = None;
                            let mut backfill = None;
                            for opt in options {
                                match opt.name.as_str() {
                                    "room_id" => {
                                        room_id_str = opt.value.as_str().to_owned();
                                    }
                                    "continuous" => {
                                        continuous = opt.value.as_bool();
                                    }
                                    "backfill" => {
                                        backfill = opt.value.as_bool();
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

                            let realm_config = RealmConfig {
                                lamprey_room_id,
                                discord_guild_id: guild_id,
                                continuous: continuous.unwrap_or(false),
                            };

                            if let Err(e) = globals.insert_realm(realm_config.clone()).await {
                                error!("failed to insert realm: {e}");
                                send_ephemeral_reply(&ctx, &command, "error: failed to link guild")
                                    .await;
                                return;
                            }

                            if backfill.unwrap_or(false) {
                                send_ephemeral_reply(
                                    &ctx,
                                    &command,
                                    "guild linked, backfilling...",
                                )
                                .await;

                                let globals = globals.clone();
                                let ctx = ctx.clone();
                                tokio::spawn(async move {
                                    if let Err(e) =
                                        backfill_guild(&ctx, globals, guild_id, realm_config).await
                                    {
                                        error!("failed to backfill guild {}: {}", guild_id, e);
                                    }
                                });
                            } else {
                                send_ephemeral_reply(&ctx, &command, "guild linked").await;
                            }
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

    async fn guild_member_update(
        &self,
        ctx: Context,
        old_if_available: Option<serenity::model::guild::Member>,
        new: Option<serenity::model::guild::Member>,
        _event: GuildMemberUpdateEvent,
    ) {
        let Some(new) = new else {
            return;
        };
        info!("discord guild member update");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap().clone();

        let old_nick = old_if_available.and_then(|m| m.nick);
        if old_nick == new.nick {
            return;
        }

        tokio::spawn(async move {
            let res: Result<()> = async {
                let Some(puppet) = globals
                    .get_puppet("discord", &new.user.id.to_string())
                    .await?
                else {
                    debug!("no puppet found for discord user {}", new.user.id);
                    return Ok(());
                };

                let realms = globals.get_realms().await?;
                let Some(realm) = realms.iter().find(|r| r.discord_guild_id == new.guild_id) else {
                    debug!("no realm found for guild {}", new.guild_id);
                    return Ok(());
                };

                let ly = globals.lamprey_handle().await?;

                let patch = common::v1::types::RoomMemberPatch {
                    override_name: Some(new.nick.clone()),
                    override_description: None,
                    mute: None,
                    deaf: None,
                    roles: None,
                    timeout_until: None,
                };

                ly.room_member_patch(realm.lamprey_room_id, puppet.id.into(), &patch)
                    .await?;
                info!("updated lamprey member nick for {}", new.user.id);
                Ok(())
            }
            .await;
            if let Err(e) = res {
                error!("failed to handle guild member update: {e}");
            }
        });
    }

    async fn presence_update(&self, ctx: Context, presence: Presence) {
        debug!("discord presence update for user {}", presence.user.id);
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap().clone();

        globals.presences.insert(presence.user.id, presence.clone());

        tokio::spawn(async move {
            if let Err(e) = process_presence_update(globals, presence).await {
                error!("failed to handle presence update: {e}");
            }
        });
    }
}

pub async fn process_presence_update(globals: Arc<Globals>, presence: Presence) -> Result<()> {
    let Some(puppet) = globals
        .get_puppet("discord", &presence.user.id.to_string())
        .await?
    else {
        trace!("no puppet found for discord user {}", presence.user.id);
        return Ok(());
    };

    let status = match presence.status {
        OnlineStatus::Online => Some(StatusTypePatch::Online),
        OnlineStatus::Idle => Some(StatusTypePatch::Away {}),
        OnlineStatus::DoNotDisturb => Some(StatusTypePatch::Busy { dnd: true }),
        OnlineStatus::Invisible | OnlineStatus::Offline => Some(StatusTypePatch::Offline),
        _ => None,
    };

    let status_text = presence
        .activities
        .iter()
        .find(|a| a.kind == ActivityType::Custom)
        .and_then(|a| a.state.clone())
        .map(|text| {
            Some(StatusText {
                text,
                clear_at: None,
            })
        });

    let patch = StatusPatch {
        status,
        status_text,
    };

    if patch.status.is_none() && patch.status_text.is_none() {
        return Ok(());
    }

    let ly = globals.lamprey_handle().await?;
    ly.user_set_status(puppet.id.into(), &patch).await?;
    info!("updated lamprey presence for {}", presence.user.id);
    Ok(())
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
        ty: common::v1::types::ChannelType,
        parent_id: Option<ChannelId>,
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
    ty: common::v1::types::ChannelType,
    parent_id: Option<serenity::all::ChannelId>,
) -> Result<serenity::all::ChannelId> {
    let (send, recv) = oneshot::channel();
    globals
        .dc_chan
        .send(DiscordMessage::ChannelCreate {
            guild_id,
            name,
            ty,
            parent_id,
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
