use std::sync::Arc;

use anyhow::{anyhow, Result};
use serenity::all::{
    ChannelType, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, CreateInteractionResponseMessage, CreateWebhook, EditInteractionResponse,
    GuildId, InteractionContext, InteractionResponseFlags, MessagePagination, Permissions, Webhook,
};
use serenity::model::prelude::{ChannelId, MessageId};
use tracing::{debug, error, info, warn};

use crate::bridge_common::{Globals, RealmConfig, WEBHOOK_NAME};
use crate::db::Data;
use crate::portal::{Portal, PortalMessage};

pub(super) async fn send_ephemeral_reply(
    ctx: &serenity::all::Context,
    command: &CommandInteraction,
    content: &str,
) {
    let builder = CreateInteractionResponseMessage::new()
        .content(content.to_string())
        .flags(InteractionResponseFlags::EPHEMERAL);
    let response = serenity::all::CreateInteractionResponse::Message(builder);
    if let Err(err) = command.create_response(&ctx.http, response).await {
        error!("failed to respond to interaction: {err:?}");
    }
}

pub(super) async fn backfill_channel(
    ctx: &serenity::all::Context,
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
            .http
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
            let _ = portal
                .send(PortalMessage::DiscordMessageCreate { message })
                .await;
        }
        p = MessagePagination::After(last_id);
    }
    info!("finished backfill for channel {}", channel_id);
    Ok(())
}

pub(super) async fn backfill_guild(
    ctx: &serenity::all::Context,
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
            let webhook = crate::discord::discord_create_webhook(
                globals.clone(),
                webhook_channel_id,
                WEBHOOK_NAME.to_string(),
            )
            .await?;
            webhook.url().unwrap().to_string()
        } else {
            "".to_string()
        };

        let portal_config = crate::bridge_common::PortalConfig {
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

pub(super) fn get_commands() -> Vec<CreateCommand> {
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

async fn handle_link_channel(
    ctx: &serenity::all::Context,
    _command: &CommandInteraction,
    channel_id: ChannelId,
    guild_id: GuildId,
    subcommand: &serenity::all::CommandDataOption,
    globals: Arc<Globals>,
) -> Result<String, String> {
    let CommandDataOptionValue::SubCommand(options) = &subcommand.value else {
        return Err("error: invalid subcommand".to_string());
    };

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

    let Some(room_id_str) = room_id_str else {
        return Err("error: missing required options".to_string());
    };
    let Some(thread_id_str) = thread_id_str else {
        return Err("error: missing required options".to_string());
    };

    let lamprey_room_id: common::v1::types::RoomId = match room_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return Err("error: invalid room id".to_string());
        }
    };

    let lamprey_thread_id: common::v1::types::ChannelId = match thread_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return Err("error: invalid thread id".to_string());
        }
    };

    let has_existing_discord = globals
        .get_portal_by_discord_channel(channel_id)
        .await
        .is_ok_and(|p| p.is_some());
    if has_existing_discord {
        return Err("error: this discord channel is already bridged".to_string());
    }

    let has_existing_lamprey = globals
        .get_portal_by_thread_id(lamprey_thread_id)
        .await
        .is_ok_and(|p| p.is_some());
    if has_existing_lamprey {
        return Err("error: that lamprey thread is already bridged".to_string());
    }

    let guild = ctx.cache.guild(guild_id).expect("guild").to_owned();
    let thread = guild.threads.iter().find(|t| t.id == channel_id).cloned();

    let webhook = match ctx
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
    {
        Ok(w) => w,
        Err(_) => {
            return Err("error: could not create webhook".to_string());
        }
    };

    let _ = globals
        .insert_portal(crate::bridge_common::PortalConfig {
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
        let ctx = ctx.clone();
        let globals = globals.clone();
        tokio::spawn(async move {
            if let Err(e) = backfill_channel(&ctx, globals, channel_id).await {
                error!("failed to backfill channel {}: {}", channel_id, e);
            }
        });
        Ok("linked, backfilling...".to_string())
    } else {
        Ok("linked".to_string())
    }
}

async fn handle_link_guild(
    ctx: &serenity::all::Context,
    _command: &CommandInteraction,
    guild_id: GuildId,
    subcommand: &serenity::all::CommandDataOption,
    globals: Arc<Globals>,
) -> Result<String, String> {
    let CommandDataOptionValue::SubCommand(options) = &subcommand.value else {
        return Err("error: invalid subcommand".to_string());
    };

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
        return Err("error: missing room_id".to_string());
    };

    let lamprey_room_id: common::v1::types::RoomId = match room_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return Err("error: invalid room id".to_string());
        }
    };

    let realm_config = RealmConfig {
        lamprey_room_id,
        discord_guild_id: guild_id,
        continuous: continuous.unwrap_or(false),
    };

    if let Err(e) = globals.insert_realm(realm_config.clone()).await {
        error!("failed to insert realm: {e}");
        return Err("error: failed to link guild".to_string());
    }

    if backfill.unwrap_or(false) {
        let globals = globals.clone();
        let ctx = ctx.clone();
        tokio::spawn(async move {
            if let Err(e) = backfill_guild(&ctx, globals, guild_id, realm_config).await {
                error!("failed to backfill guild {}: {}", guild_id, e);
            }
        });
        Ok("guild linked, backfilling...".to_string())
    } else {
        Ok("guild linked".to_string())
    }
}

async fn handle_unlink_channel(
    ctx: &serenity::all::Context,
    _command: &CommandInteraction,
    channel_id: ChannelId,
    _guild_id: GuildId,
    globals: Arc<Globals>,
) -> Result<String, String> {
    let Some(portal) = globals
        .get_portal_by_discord_channel(channel_id)
        .await
        .ok()
        .flatten()
    else {
        return Err("this channel isnt bridged".to_string());
    };

    if let Ok(w) = Webhook::from_url(&ctx.http, &portal.discord_webhook).await {
        if w.delete(&ctx.http).await.is_err() {
            warn!("failed to delete webhook when unlinking channel");
        }
    }

    match globals.delete_portal(portal.lamprey_thread_id).await {
        Ok(_) => Ok("done".to_string()),
        Err(err) => {
            error!("failed to unlink: {err:?}");
            Err("failed to unlink, see logs for info".to_string())
        }
    }
}

async fn handle_unlink_guild(
    _ctx: &serenity::all::Context,
    _command: &CommandInteraction,
    guild_id: GuildId,
    globals: Arc<Globals>,
) -> Result<String, String> {
    let realms = match globals.get_realms().await {
        Ok(r) => r,
        Err(e) => {
            error!("failed to get realms: {e}");
            return Err("error: failed to get realms".to_string());
        }
    };

    let Some(realm) = realms.iter().find(|r| r.discord_guild_id == guild_id) else {
        return Err("error: this guild is not linked".to_string());
    };

    if let Err(e) = globals.delete_realm(realm.lamprey_room_id).await {
        error!("failed to delete realm: {e}");
        return Err("error: failed to unlink guild".to_string());
    }

    Ok("guild unlinked".to_string())
}

pub(super) async fn handle_interaction(
    ctx: &serenity::all::Context,
    command: CommandInteraction,
    globals: Arc<Globals>,
) {
    match command.data.name.as_str() {
        "ping" => {
            send_ephemeral_reply(ctx, &command, "pong!").await;
        }
        "link" => {
            // Defer immediately to avoid Discord's 3-second timeout
            if let Err(e) = command.defer(&ctx.http).await {
                error!("failed to defer link command: {e}");
                return;
            }

            let guild_id = command.guild_id.unwrap();
            let channel_id = command.channel_id;

            let subcommand = &command.data.options[0];
            let result = match subcommand.name.as_str() {
                "channel" => {
                    handle_link_channel(
                        ctx,
                        &command,
                        channel_id,
                        guild_id,
                        subcommand,
                        globals.clone(),
                    )
                    .await
                }
                "guild" => {
                    handle_link_guild(ctx, &command, guild_id, subcommand, globals.clone()).await
                }
                _ => Err("error: unknown subcommand".to_string()),
            };

            // Edit the deferred response with the result
            let response_msg = match result {
                Ok(msg) => msg,
                Err(err) => err,
            };
            let builder = EditInteractionResponse::new().content(response_msg);
            if let Err(e) = command.edit_response(&ctx.http, builder).await {
                error!("failed to edit interaction response: {e}");
            }
        }
        "unlink" => {
            // Defer immediately to avoid Discord's 3-second timeout
            if let Err(e) = command.defer(&ctx.http).await {
                error!("failed to defer unlink command: {e}");
                return;
            }

            let guild_id = command.guild_id.unwrap();
            let channel_id = command.channel_id;

            let subcommand = &command.data.options[0];
            let result = match subcommand.name.as_str() {
                "channel" => {
                    handle_unlink_channel(ctx, &command, channel_id, guild_id, globals.clone())
                        .await
                }
                "guild" => handle_unlink_guild(ctx, &command, guild_id, globals.clone()).await,
                _ => Err("error: unknown subcommand".to_string()),
            };

            // Edit the deferred response with the result
            let response_msg = match result {
                Ok(msg) => msg,
                Err(err) => err,
            };
            let builder = EditInteractionResponse::new().content(response_msg);
            if let Err(e) = command.edit_response(&ctx.http, builder).await {
                error!("failed to edit interaction response: {e}");
            }
        }
        _ => {}
    }
}
