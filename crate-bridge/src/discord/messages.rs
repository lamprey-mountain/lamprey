use std::sync::Arc;

use anyhow::Result;
use serenity::all::{ChannelId, GuildId, MessageId, Webhook};

use crate::bridge_common::Globals;
use crate::discord::{DiscordMessage, DiscordResponse};

pub async fn discord_create_channel(
    globals: Arc<Globals>,
    guild_id: GuildId,
    name: String,
    ty: common::v1::types::ChannelType,
    parent_id: Option<serenity::all::ChannelId>,
) -> Result<serenity::all::ChannelId> {
    let discord = globals.get_discord()?;
    let response = discord
        .handle_message(DiscordMessage::ChannelCreate {
            guild_id,
            name,
            ty,
            parent_id,
        })
        .await?;

    match response {
        DiscordResponse::ChannelId(id) => Ok(id),
        _ => Err(anyhow::anyhow!(
            "unexpected response type from Discord actor"
        )),
    }
}

pub async fn discord_create_webhook(
    globals: Arc<Globals>,
    channel_id: serenity::all::ChannelId,
    name: String,
) -> Result<Webhook> {
    let discord = globals.get_discord()?;
    let response = discord
        .handle_message(DiscordMessage::WebhookCreate { channel_id, name })
        .await?;

    match response {
        DiscordResponse::Webhook(hook) => Ok(hook),
        _ => Err(anyhow::anyhow!(
            "unexpected response type from Discord actor"
        )),
    }
}

pub async fn discord_get_message(
    globals: Arc<Globals>,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<serenity::all::Message> {
    let discord = globals.get_discord()?;
    let response = discord
        .handle_message(DiscordMessage::MessageGet {
            message_id,
            channel_id,
        })
        .await?;

    match response {
        DiscordResponse::Message(msg) => Ok(msg),
        _ => Err(anyhow::anyhow!(
            "unexpected response type from Discord actor"
        )),
    }
}

pub async fn discord_execute_webhook(
    globals: Arc<Globals>,
    url: String,
    payload: serenity::all::ExecuteWebhook,
) -> Result<serenity::all::Message> {
    let discord = globals.get_discord()?;
    let response = discord
        .handle_message(DiscordMessage::WebhookExecute { url, payload })
        .await?;

    match response {
        DiscordResponse::Message(msg) => Ok(msg),
        _ => Err(anyhow::anyhow!(
            "unexpected response type from Discord actor"
        )),
    }
}

pub async fn discord_edit_message(
    globals: Arc<Globals>,
    url: String,
    message_id: MessageId,
    payload: serenity::all::EditWebhookMessage,
) -> Result<serenity::all::Message> {
    let discord = globals.get_discord()?;
    let response = discord
        .handle_message(DiscordMessage::WebhookMessageEdit {
            url,
            message_id,
            payload,
        })
        .await?;

    match response {
        DiscordResponse::Message(msg) => Ok(msg),
        _ => Err(anyhow::anyhow!(
            "unexpected response type from Discord actor"
        )),
    }
}

pub async fn discord_delete_message(
    globals: Arc<Globals>,
    url: String,
    thread_id: Option<ChannelId>,
    message_id: MessageId,
) -> Result<()> {
    let discord = globals.get_discord()?;
    let response = discord
        .handle_message(DiscordMessage::WebhookMessageDelete {
            url,
            thread_id,
            message_id,
        })
        .await?;

    match response {
        DiscordResponse::Unit => Ok(()),
        _ => Err(anyhow::anyhow!(
            "unexpected response type from Discord actor"
        )),
    }
}
