// TODO(#874) split out channel create structs

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    v1::types::{ChannelType, MessageCreate, PermissionOverwrite},
    v2::types::{ChannelId, MediaId, TagId, UserId},
};

/// data needed to create a new channel in a room
///
/// threads can't be created with this endpoint
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelCreateRoom {
    /// The type of channel to create
    ///
    /// Must not be a thread or dm type channel
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    pub name: Option<String>,
    pub description: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub nsfw: bool,
    pub bitrate: Option<u64>,
    pub user_limit: Option<u64>,
    pub parent_id: Option<ChannelId>,

    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 64)))]
    pub permission_overwrites: Vec<PermissionOverwrite>,

    pub auto_archive_duration: Option<u64>,
    pub default_auto_archive_duration: Option<u64>,
    pub slowmode_thread: Option<u64>,
    pub slowmode_message: Option<u64>,
    pub default_slowmode_message: Option<u64>,
}

/// Data needed to create a new dm or gdm
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelCreateDm {
    /// The type of channel to create
    ///
    /// Must be `Dm` or `Gdm`
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<MediaId>,

    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 16)))]
    pub recipients: Option<Vec<UserId>>,
}

/// Data needed to create a new thread
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadCreate {
    /// The type of channel to create
    ///
    /// Must be `ThreadPublic`, `ThreadForum2`, or `ThreadPrivate` depending on the parent channel
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    pub name: String,
    pub description: Option<String>,

    /// Tags to apply, only usable in forums
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Option<Vec<TagId>>,

    /// The initial message for this thread. Required for forum/thread-only channels.
    pub starter_message: Option<Box<MessageCreate>>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub invitable: bool,
    pub auto_archive_duration: Option<u64>,
    pub slowmode_message: Option<u64>,
}

/// Data needed to create a new thread from a message
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadCreateFromMessage {
    /// The type of channel to create
    ///
    /// Must be `ThreadPublic`
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    pub name: Option<String>,
    pub description: Option<String>,
    pub auto_archive_duration: Option<u64>,
    pub slowmode_message: Option<u64>,
}

