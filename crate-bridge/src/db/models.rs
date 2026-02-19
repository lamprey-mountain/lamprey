use anyhow::Result;
use common::v1::types::{ChannelId, MediaId, MessageId};
use serenity::all::{
    AttachmentId as DcAttachmentId, ChannelId as DcChannelId, MessageId as DcMessageId,
};
use uuid::Uuid;

use crate::bridge_common::{PortalConfig, RealmConfig};

pub(super) struct PortalConfigRow {
    pub lamprey_thread_id: String,
    pub lamprey_room_id: String,
    pub discord_guild_id: String,
    pub discord_channel_id: String,
    pub discord_thread_id: Option<String>,
    pub discord_webhook: String,
}

impl TryFrom<PortalConfigRow> for PortalConfig {
    type Error = anyhow::Error;

    fn try_from(value: PortalConfigRow) -> Result<Self, Self::Error> {
        Ok(Self {
            lamprey_thread_id: value.lamprey_thread_id.parse()?,
            lamprey_room_id: value.lamprey_room_id.parse()?,
            discord_guild_id: value.discord_guild_id.parse()?,
            discord_channel_id: value.discord_channel_id.parse()?,
            discord_thread_id: value.discord_thread_id.map(|v| v.parse()).transpose()?,
            discord_webhook: value.discord_webhook,
        })
    }
}

impl From<PortalConfig> for PortalConfigRow {
    fn from(value: PortalConfig) -> Self {
        Self {
            lamprey_thread_id: value.lamprey_thread_id.to_string(),
            lamprey_room_id: value.lamprey_room_id.to_string(),
            discord_guild_id: value.discord_guild_id.to_string(),
            discord_channel_id: value.discord_channel_id.to_string(),
            discord_thread_id: value.discord_thread_id.map(|v| v.to_string()),
            discord_webhook: value.discord_webhook,
        }
    }
}

pub struct MessageMetadata {
    pub chat_id: MessageId,
    pub chat_thread_id: ChannelId,
    pub discord_id: DcMessageId,
    /// the THREAD id, falling back to channel id
    pub discord_channel_id: DcChannelId,
}

pub(super) struct MessageMetadataRow {
    pub chat_id: String,
    pub chat_thread_id: String,
    pub discord_id: String,
    pub discord_channel_id: String,
}

pub struct AttachmentMetadata {
    pub chat_id: MediaId,
    pub discord_id: DcAttachmentId,
}

pub(super) struct AttachmentMetadataRow {
    pub chat_id: String,
    pub discord_id: String,
}

#[derive(Debug)]
pub struct Puppet {
    pub id: Uuid,
    pub ext_platform: String,
    pub ext_id: String,
    pub ext_avatar: Option<String>,
    pub ext_banner: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    // TODO: remove Option
    pub bot: Option<bool>,
}

pub(super) struct RealmConfigRow {
    pub lamprey_room_id: String,
    pub discord_guild_id: String,
    pub continuous: bool,
}

impl TryFrom<RealmConfigRow> for RealmConfig {
    type Error = anyhow::Error;

    fn try_from(value: RealmConfigRow) -> Result<Self> {
        Ok(Self {
            lamprey_room_id: value.lamprey_room_id.parse()?,
            discord_guild_id: value.discord_guild_id.parse()?,
            continuous: value.continuous,
        })
    }
}

impl From<RealmConfig> for RealmConfigRow {
    fn from(value: RealmConfig) -> Self {
        Self {
            lamprey_room_id: value.lamprey_room_id.to_string(),
            discord_guild_id: value.discord_guild_id.to_string(),
            continuous: value.continuous,
        }
    }
}

impl TryFrom<MessageMetadataRow> for MessageMetadata {
    type Error = anyhow::Error;

    fn try_from(row: MessageMetadataRow) -> Result<Self> {
        Ok(MessageMetadata {
            chat_id: row.chat_id.parse()?,
            chat_thread_id: row.chat_thread_id.parse()?,
            discord_id: row.discord_id.parse()?,
            discord_channel_id: row.discord_channel_id.parse()?,
        })
    }
}

impl From<MessageMetadata> for MessageMetadataRow {
    fn from(value: MessageMetadata) -> Self {
        MessageMetadataRow {
            chat_id: value.chat_id.to_string(),
            chat_thread_id: value.chat_thread_id.to_string(),
            discord_id: value.discord_id.to_string(),
            discord_channel_id: value.discord_channel_id.to_string(),
        }
    }
}

impl TryFrom<AttachmentMetadataRow> for AttachmentMetadata {
    type Error = anyhow::Error;

    fn try_from(row: AttachmentMetadataRow) -> Result<Self> {
        Ok(Self {
            chat_id: row.chat_id.parse()?,
            discord_id: row.discord_id.parse()?,
        })
    }
}

impl From<AttachmentMetadata> for AttachmentMetadataRow {
    fn from(value: AttachmentMetadata) -> Self {
        Self {
            chat_id: value.chat_id.to_string(),
            discord_id: value.discord_id.to_string(),
        }
    }
}
