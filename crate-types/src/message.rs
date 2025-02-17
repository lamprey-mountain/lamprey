use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::util::Diff;

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
use crate::util::some_option;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePatch {
    #[serde(default, deserialize_with = "some_option")]
    pub content: Option<Option<String>>,

    pub attachments: Option<Vec<MediaRef>>,

    #[serde(default, deserialize_with = "some_option")]
    pub metadata: Option<Option<serde_json::Value>>,

    #[serde(default, deserialize_with = "some_option")]
    pub reply_id: Option<Option<MessageId>>,

    // is this temporary, or should i keep it?
    // removing it would break all existing bridged messages
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageType {
    /// a basic message
    Default,

    /// a message logging an update to the thread
    ThreadUpdate,
}

impl Diff<Message> for MessagePatch {
    fn changes(&self, other: &Message) -> bool {
        self.content.changes(&other.content)
            || self.metadata.changes(&other.metadata)
            || self.reply_id.changes(&other.reply_id)
            || self.override_name.changes(&other.override_name)
            || self.attachments.as_ref().is_some_and(|a| {
                a.len() != other.attachments.len()
                    || a.iter().zip(&other.attachments).any(|(a, b)| a.id != b.id)
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
