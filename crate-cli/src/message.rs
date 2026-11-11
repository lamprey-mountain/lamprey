use std::fmt;

use anyhow::Result;
use common::v1::types::{ChannelId, Message, MessageCreate, MessageId, MessageType};
use serde::Serialize;

use crate::auth;
use crate::config::LyConfig;

/// result of `ly message send` and `ly message get`
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct MessageResult(pub Message);

/// result of `ly message delete`
#[derive(Debug, Serialize)]
pub struct DeleteResult {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

impl fmt::Display for MessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = &self.0;
        write!(
            f,
            "{} ({}): {}",
            message.id,
            message.author_id,
            content(message).unwrap_or("(no text content)")
        )
    }
}

impl fmt::Display for DeleteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "deleted message {} in {}",
            self.message_id, self.channel_id
        )
    }
}

/// send a message
pub async fn send(
    config: &mut LyConfig,
    login: Option<&str>,
    channel_id: ChannelId,
    content: String,
    reply_to: Option<MessageId>,
) -> Result<MessageResult> {
    let http = auth::client(config, login).await?;
    let message = http
        .message_create(
            channel_id,
            &MessageCreate {
                content: Some(content),
                reply_id: reply_to,
                ..Default::default()
            },
        )
        .await?;

    Ok(MessageResult(message))
}

/// get a specific message
pub async fn get(
    config: &mut LyConfig,
    login: Option<&str>,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<MessageResult> {
    let http = auth::client(config, login).await?;
    let message = http.message_get(channel_id, message_id).await?;

    Ok(MessageResult(message))
}

/// delete a message
pub async fn delete(
    config: &mut LyConfig,
    login: Option<&str>,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<DeleteResult> {
    let http = auth::client(config, login).await?;
    http.message_delete(channel_id, message_id).await?;

    Ok(DeleteResult {
        channel_id,
        message_id,
    })
}

/// the text content of a message, if it has any
fn content(message: &Message) -> Option<&str> {
    match &message.latest_version.message_type {
        MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => m.content.as_deref(),
        _ => None,
    }
}
