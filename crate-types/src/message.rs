use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{Media, MediaRef, MessageId, MessageVerId, ThreadId, User};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageCreateRequest {
    pub content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<MediaRef>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<MessageId>,
    pub override_name: Option<String>, // temp?
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePatch {
    pub content: Option<Option<String>>,
    pub attachments: Option<Vec<MediaRef>>,
    pub metadata: Option<Option<serde_json::Value>>,
    pub reply_id: Option<Option<MessageId>>,
    pub override_name: Option<Option<String>>, // temp?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageType {
    Default,
    ThreadUpdate,
}

impl MessagePatch {
    pub fn wont_change(&self, target: &Message) -> bool {
        self.content.as_ref().is_none_or(|c| c == &target.content)
            && self.metadata.as_ref().is_none_or(|m| m == &target.metadata)
            && self.reply_id.is_none_or(|r| r == target.reply_id)
            && self
                .override_name
                .as_ref()
                .is_none_or(|o| o == &target.override_name)
            && self.attachments.as_ref().is_none_or(|a| {
                a.len() == target.attachments.len()
                    && a.iter().zip(&target.attachments).all(|(a, b)| a.id == b.id)
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

    pub fn is_editable(&self) -> bool {
        match self {
            MessageType::Default => true,
            MessageType::ThreadUpdate => false,
        }
    }
}
