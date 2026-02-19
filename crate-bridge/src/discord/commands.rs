use std::sync::Arc;

use anyhow::{anyhow, Result};
use serenity::all::{
    ChannelType, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, CreateInteractionResponseMessage, CreateWebhook, GuildId,
    InteractionContext, InteractionResponseFlags, MessagePagination, Permissions, Webhook,
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
            let _ = portal.send(PortalMessage::DiscordMessageCreate { message });
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

                        let (Some(room_id_str), Some(thread_id_str)) = (room_id_str, thread_id_str)
                        else {
                            send_ephemeral_reply(ctx, &command, "error: missing required options")
                                .await;
                            return;
                        };

                        let lamprey_room_id: common::v1::types::RoomId = match room_id_str.parse() {
                            Ok(id) => id,
                            Err(_) => {
                                send_ephemeral_reply(ctx, &command, "error: invalid room id").await;
                                return;
                            }
                        };

                        let lamprey_thread_id: common::v1::types::ChannelId =
                            match thread_id_str.parse() {
                                Ok(id) => id,
                                Err(_) => {
                                    send_ephemeral_reply(ctx, &command, "error: invalid thread id")
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
                                ctx,
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
                                ctx,
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
                            send_ephemeral_reply(ctx, &command, "error: could not create webhook")
                                .await;
                            return;
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
                            send_ephemeral_reply(ctx, &command, "linked, backfilling...").await;
                            let ctx = ctx.clone();
                            let globals = globals.clone();
                            tokio::spawn(async move {
                                if let Err(e) = backfill_channel(&ctx, globals, channel_id).await {
                                    error!("failed to backfill channel {}: {}", channel_id, e);
                                }
                            });
                        } else {
                            send_ephemeral_reply(ctx, &command, "linked").await;
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
                            send_ephemeral_reply(ctx, &command, "error: missing room_id").await;
                            return;
                        };

                        let lamprey_room_id: common::v1::types::RoomId = match room_id_str.parse() {
                            Ok(id) => id,
                            Err(_) => {
                                send_ephemeral_reply(ctx, &command, "error: invalid room id").await;
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
                            send_ephemeral_reply(ctx, &command, "error: failed to link guild")
                                .await;
                            return;
                        }

                        if backfill.unwrap_or(false) {
                            send_ephemeral_reply(ctx, &command, "guild linked, backfilling...")
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
                            send_ephemeral_reply(ctx, &command, "guild linked").await;
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
                        if let Ok(w) = Webhook::from_url(&ctx.http, &portal.discord_webhook).await {
                            if w.delete(&ctx.http).await.is_err() {
                                send_ephemeral_reply(
                                    ctx,
                                    &command,
                                    "warning: could not delete webhook",
                                )
                                .await;
                            }
                        }

                        match globals.delete_portal(portal.lamprey_thread_id).await {
                            Ok(_) => {
                                send_ephemeral_reply(ctx, &command, "done").await;
                            }
                            Err(err) => {
                                error!("failed to unlink: {err:?}");
                                send_ephemeral_reply(
                                    ctx,
                                    &command,
                                    "failed to unlink, see logs for info",
                                )
                                .await;
                            }
                        }
                    } else {
                        send_ephemeral_reply(ctx, &command, "this channel isnt bridged").await;
                    }
                }
                "guild" => {
                    let realms = match globals.get_realms().await {
                        Ok(r) => r,
                        Err(e) => {
                            error!("failed to get realms: {e}");
                            send_ephemeral_reply(ctx, &command, "error: failed to get realms")
                                .await;
                            return;
                        }
                    };

                    let Some(realm) = realms.iter().find(|r| r.discord_guild_id == guild_id) else {
                        send_ephemeral_reply(ctx, &command, "error: this guild is not linked")
                            .await;
                        return;
                    };

                    if let Err(e) = globals.delete_realm(realm.lamprey_room_id).await {
                        error!("failed to delete realm: {e}");
                        send_ephemeral_reply(ctx, &command, "error: failed to unlink guild").await;
                        return;
                    }

                    send_ephemeral_reply(ctx, &command, "guild unlinked").await;
                }
                _ => {}
            }
        }
        _ => {}
    }
}
