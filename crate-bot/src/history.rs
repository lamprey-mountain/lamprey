//! llm conversation history, built on [`Store`].

use anyhow::Result;
use common::v2::types::ChannelId;

use crate::llm::ChatMessage;
use crate::store::Store;

/// namespace shared by every llm history key
const LLM_PREFIX: &str = "llm";

/// key under which the next message sequence number is stored
const NEXT_SEQ_KEY: &str = "llm.next_seq";

/// key for a single message: `llm.<channel>.<seq>`
type MessageKey = (String, ChannelId, String);

/// prefix matching every message in a channel's conversation
type ConversationKey = (String, ChannelId);

/// build the store key for a single message
fn message_key(channel_id: ChannelId, seq: u64) -> MessageKey {
    (LLM_PREFIX.to_string(), channel_id, seq.to_string())
}

/// prefix matching every message in `channel_id`'s conversation
fn conversation_prefix(channel_id: ChannelId) -> ConversationKey {
    (LLM_PREFIX.to_string(), channel_id)
}

/// get the next message sequence number
async fn next_seq(store: &Store) -> Result<u64> {
    let current: Option<u64> = store.get(NEXT_SEQ_KEY.to_string()).await?;
    let next = current.unwrap_or(0) + 1;
    store.put(NEXT_SEQ_KEY.to_string(), &next).await?;
    Ok(next)
}

/// append a message to `channel_id`'s conversation
pub async fn append(store: &Store, channel_id: ChannelId, message: &ChatMessage) -> Result<()> {
    let seq = next_seq(store).await?;
    store.put(message_key(channel_id, seq), message).await
}

/// whether `channel_id` already has a conversation
pub async fn exists(store: &Store, channel_id: ChannelId) -> Result<bool> {
    // maybe reuse list() for this
    let prefix = conversation_prefix(channel_id);
    let messages: Vec<(MessageKey, ChatMessage)> = store.all(&prefix).await?;
    Ok(!messages.is_empty())
}

/// load `channel_id`'s conversation, oldest message first
pub async fn list(store: &Store, channel_id: ChannelId) -> Result<Vec<ChatMessage>> {
    let prefix = conversation_prefix(channel_id);
    let mut messages: Vec<(MessageKey, ChatMessage)> = store.all(&prefix).await?;
    messages.sort_by_key(|(key, _)| key.2.parse::<u64>().unwrap_or(0));
    Ok(messages.into_iter().map(|(_, message)| message).collect())
}

// TODO: handle message edits/deletes
// TODO: resync message history when the bot (re)connects
