//! Discord synchronization and backfill utilities

use std::sync::Arc;

use anyhow::Result;
use kameo::actor::Spawn;
use kameo::prelude::ActorRef;
use serenity::all::{ChannelId, Context, GuildId, MessageId, MessagePagination};
use tracing::{debug, error, info, warn};

use crate::bridge_common::Globals;
use crate::db::Data;
use crate::portal::{Portal, PortalMessage};

/// Backfill messages from Discord to Lamprey for a single channel
pub async fn backfill_discord_channel(
    ctx: &Context,
    globals: Arc<Globals>,
    channel_id: ChannelId,
    portal: ActorRef<Portal>,
) -> Result<()> {
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
                .tell(PortalMessage::DiscordMessageCreate { message })
                .await;
        }
        p = MessagePagination::After(last_id);
    }
    info!("finished backfill for channel {}", channel_id);
    Ok(())
}

/// Backfill messages from Discord to Lamprey for a channel that already has a portal
/// Used by incremental backfill in events.rs
pub async fn backfill_discord_channel_incremental(
    ctx: &Context,
    globals: Arc<Globals>,
    channel_id: ChannelId,
    portal: ActorRef<Portal>,
) -> Result<()> {
    let last_id = globals
        .last_discord_ids
        .get(&channel_id)
        .map(|v| *v.value());
    let Some(last_id) = last_id else {
        warn!("no last_id for channel {}, skipping backfill", channel_id);
        return Ok(());
    };

    let mut p = MessagePagination::After(last_id);
    loop {
        let msgs = ctx
            .http
            .get_messages(channel_id, Some(p), Some(100))
            .await?;

        if msgs.is_empty() {
            break;
        }

        info!("discord backfill {} messages", msgs.len());
        let last_id = msgs.first().unwrap().id;
        for message in msgs.into_iter().rev() {
            if globals.get_message_dc(message.id).await?.is_some() {
                debug!("skipping already bridged message: {}", message.id);
                continue;
            }
            let _ = portal
                .tell(PortalMessage::DiscordMessageCreate { message })
                .await;
        }
        p = MessagePagination::After(last_id);
    }
    Ok(())
}

/// Backfill all channels in a Discord guild
pub async fn backfill_discord_guild(
    ctx: &Context,
    globals: Arc<Globals>,
    guild_id: GuildId,
) -> Result<()> {
    let guild = ctx
        .cache
        .guild(guild_id)
        .ok_or_else(|| anyhow::anyhow!("failed to get guild {guild_id} from cache"))?
        .to_owned();

    let mut all_channels: Vec<_> = guild.channels.values().chain(&guild.threads).collect();
    all_channels.sort_by_key(|c| c.parent_id.is_some());

    for channel in all_channels {
        if !matches!(
            channel.kind,
            serenity::all::ChannelType::Text
                | serenity::all::ChannelType::News
                | serenity::all::ChannelType::PublicThread
                | serenity::all::ChannelType::PrivateThread
                | serenity::all::ChannelType::NewsThread
                | serenity::all::ChannelType::Category
        ) {
            continue;
        }

        if globals
            .get_portal_by_discord_channel(channel.id)
            .await?
            .is_some()
        {
            let ctx = ctx.clone();
            let globals = globals.clone();
            let channel_id = channel.id;
            tokio::spawn(async move {
                if let Err(e) = backfill_channel_task(&ctx, globals, channel_id).await {
                    error!(
                        "failed to backfill existing portal for channel {}: {}",
                        channel_id, e
                    );
                }
            });
            continue;
        }

        // Portal doesn't exist yet - will be created by caller
        // This function just handles the backfill of existing portals
    }

    Ok(())
}

/// Task to backfill a single channel (spawned as async task)
async fn backfill_channel_task(
    ctx: &Context,
    globals: Arc<Globals>,
    channel_id: ChannelId,
) -> Result<()> {
    let Some(config) = globals.get_portal_by_discord_channel(channel_id).await? else {
        warn!("backfill_channel_task: no portal for {}", channel_id);
        return Ok(());
    };

    let portal_ref = globals
        .portals
        .entry(config.lamprey_thread_id)
        .or_insert_with(|| Portal::spawn((globals.clone(), config.to_owned())));
    let portal = portal_ref.clone();
    drop(portal_ref); // Release the borrow before calling backfill

    backfill_discord_channel(ctx, globals, channel_id, portal).await
}

/// Ensure a portal exists for a channel, creating one if necessary
pub async fn ensure_portal_for_channel(
    globals: Arc<Globals>,
    channel: &serenity::all::GuildChannel,
    guild_id: GuildId,
) -> Result<Option<ActorRef<Portal>>> {
    if let Some(config) = globals.get_portal_by_discord_channel(channel.id).await? {
        let portal_ref = globals
            .portals
            .entry(config.lamprey_thread_id)
            .or_insert_with(|| Portal::spawn((globals.clone(), config.to_owned())));
        return Ok(Some(portal_ref.clone()));
    }

    // Portal doesn't exist - check if we should create one
    let realms = globals.get_realms().await?;

    let Some(realm_config) = realms.iter().find(|c| c.discord_guild_id == guild_id) else {
        return Ok(None);
    };

    if !realm_config.continuous {
        return Ok(None);
    }

    // Portal will be created by Bridge actor
    Ok(None)
}
