//! Types for acknowledgment operations.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{ChannelId, MessageId, MessageVerId};

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
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AckBulkItem {
    /// The id of the channel being acknowledged.
    pub channel_id: ChannelId,

    /// The last read message id. Will be resolved from version_id if empty.
    // TODO: use instead of version_id?
    pub message_id: Option<MessageId>,

    /// The last read message vewsion id in this channel.
    pub version_id: MessageVerId,

    /// The new mention count. Defaults to 0.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mention_count: u64,
}

/// Request to acknowledge a single channel/message pair.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AckReq {
    /// The last read message id. Will be resolved from version_id if empty.
    // TODO: use instead of version_id?
    pub message_id: Option<MessageId>,

    /// The last read message vewsion id in this channel.
    pub version_id: MessageVerId,

    /// The new mention count. Defaults to 0.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mention_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AckRes {
    /// The last read message id
    pub message_id: MessageId,

    /// The last read id in this channel. Currently unused, may be deprecated later?.
    pub version_id: MessageVerId,
}
