//! Types for acknowledgment operations.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{Channel, ChannelId, MessageId, misc::Time};

/// acknowledge a message in a channel
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AckCreate {
    /// The last read message id. The latest message id will be used if empty.
    pub message_id: Option<MessageId>,

    /// The new mention count. Defaults to 0.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mention_count: u64,
}

/// acknowledge many things at once
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AckBulk {
    #[cfg_attr(feature = "validator", validate(length(max = 1024)))]
    pub acks: Vec<AckBulkItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AckBulkItem {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: AckType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AckType {
    /// an acknowledgement for a message
    ///
    /// messages get marked as unread whenever a new message is sent. edits and deletes don't have any effect.
    Message {
        channel_id: ChannelId,
        message_id: MessageId,

        #[cfg_attr(feature = "serde", serde(default))]
        mention_count: u64,
    },

    /// an acknowledgement for a channel's pinned messages
    ///
    /// pins get marked unread whenever a new message is pinned. unpins and reorders don't have any effect.
    Pins { channel_id: ChannelId },
    // TODO: more ack types
    // /// an acknowledgement for your inbox (notification list)
    // Inbox,

    // /// an acknowledgement for a document
    // ///
    // /// documents get marked unread when edited, debounced(?)
    // Document {
    //     channel_id: ChannelId,
    // },

    // CalendarEvent,
}

/// a user's read state for a resource
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AckState {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: AckType,

    /// whether this is considered unread
    pub unread: bool,
    // NOTE: unsure if i should add this
    // /// when this resource was last viewed
    // pub last_viewed: Option<Time>,
}

// TODO: maybe create an AckStateChannel struct which combines Message and Pins read state

/// relevant read state metadata for a channel
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelAckMetadata {
    /// the id of the last message that was sent
    ///
    /// is `None` if no messages were sent
    pub last_message_id: Option<MessageId>,

    /// the time when the last message was pinned
    ///
    /// is `None` if no messages were pinned
    pub last_pin_timestamp: Option<Time>,

    /// the number of messages in this channel
    pub message_count: u64,

    /// the number of root messages in this channel
    ///
    /// ie. messages that don't reply to any other messages
    pub root_message_count: u64,
}

impl ChannelAckMetadata {
    /// apply this metadata to a channel
    pub fn apply(&self, channel: &mut Channel) {
        // channel.last_version_id = ...; // TODO: remove
        channel.last_message_id = self.last_message_id;
        channel.message_count = Some(self.message_count);
        channel.root_message_count = Some(self.root_message_count);
    }

    pub fn from_channel(channel: &Channel) -> Self {
        Self {
            last_message_id: channel.last_message_id,
            last_pin_timestamp: channel.last_pin_timestamp,
            message_count: channel.message_count.unwrap_or(0),
            root_message_count: channel.root_message_count.unwrap_or(0),
        }
    }
}

impl AckState {
    /// apply this read state to a channel
    pub fn apply(&self, channel: &mut Channel) {
        match &self.ty {
            AckType::Message {
                channel_id: _,
                message_id,
                mention_count,
            } => {
                channel.is_unread = Some(self.unread);
                channel.last_read_id = Some(*message_id);
                channel.mention_count = Some(*mention_count);
            }
            _ => {}
        }
    }
}
