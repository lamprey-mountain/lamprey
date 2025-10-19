use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{notifications::NotifsChannel, MessageVerId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeChatPublic {
    pub last_version_id: Option<MessageVerId>,
    pub message_count: u64,
    // maybe? pub user_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeChatPrivate {
    pub is_unread: bool,
    pub last_read_id: Option<MessageVerId>,
    pub mention_count: u64,
    // being able to have an exact unread count would be nice, but would be hard
    // to implement efficiently. if someone marks a very old message as unread,
    // i don't want to hang while counting potentially thousands of messages!
    // pub unread_count: u64,
    pub notifications: NotifsChannel,
}
