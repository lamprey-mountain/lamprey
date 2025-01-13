use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Media, MediaId, MediaRef, MessageId, MessageVerId, ThreadId, User, UserId};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub version_id: MessageVerId,
    pub nonce: Option<String>,
    pub ordering: i32,
    pub content: Option<String>,
    pub attachments: Vec<Media>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<MessageId>,
    // embeds: Embed.array().default([]),
    // mentions_users: UserId.array(),
    // mentions_roles: RoleId.array(),
    // mentions_everyone: z.boolean(),
    // resolve everything here?
    // mentions_threads: ThreadId.array(),
    // mentions_rooms: ThreadId.array(),
    // author: Member, // TODO: future? how to represent users who have left?
    pub override_name: Option<String>, // temp?
    pub author: User,
    pub is_pinned: bool,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MessageCreateRequest {
    pub content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<MediaRef>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<MessageId>,
    pub override_name: Option<String>, // temp?
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MessagePatch {
    pub content: Option<Option<String>>,
    pub attachments: Option<Vec<MediaRef>>,
    pub metadata: Option<Option<serde_json::Value>>,
    pub reply_id: Option<Option<MessageId>>,
    pub override_name: Option<Option<String>>, // temp?
}

#[derive(Debug, PartialEq, Eq)]
pub struct MessageCreate {
    pub message_type: MessageType,
    pub thread_id: ThreadId,
    pub content: Option<String>,
    pub attachment_ids: Vec<MediaId>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<MessageId>,
    pub author_id: UserId,
    pub override_name: Option<String>, // temp?
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "message_type")]
pub enum MessageType {
    Default,
    ThreadUpdate,
}

impl MessagePatch {
    pub fn wont_change(&self, other: &Message) -> bool {
        self.content.as_ref().is_none_or(|c| c == &other.content)
            && self.metadata.as_ref().is_none_or(|m| m == &other.metadata)
            && self.reply_id.is_none_or(|r| r == other.reply_id)
            && self.override_name.as_ref().is_none_or(|o| o == &other.override_name)
            && self.attachments.as_ref().is_none_or(|a| {
                a.len() == other.attachments.len()
                    && a.iter().zip(&other.attachments).all(|(a, b)| a.id == b.id)
            })
    }
}

impl MessageType {
    pub fn is_deletable(&self) -> bool {
        match self {
            MessageType::Default => true,
            MessageType::ThreadUpdate => false,
        }
    }
}
