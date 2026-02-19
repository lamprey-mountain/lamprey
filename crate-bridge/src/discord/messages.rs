use std::sync::Arc;

use anyhow::Result;
use serenity::all::EditWebhookMessage;
use serenity::all::{ChannelId, ExecuteWebhook, GuildId, Message, MessageId, Webhook};
use tokio::sync::oneshot;

use crate::bridge_common::Globals;

/// discord actor message
#[derive(Debug)]
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
