use anyhow::Result;
use async_trait::async_trait;
use common::v1::types::{MediaId, MessageId, RoomId, ThreadId};
use serenity::all::{
    AttachmentId as DcAttachmentId, ChannelId as DcChannelId, GuildId as DcGuildId,
    MessageId as DcMessageId,
};
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::common::Globals;

#[derive(Debug, Clone)]
pub struct PortalConfig {
    pub lamprey_thread_id: ThreadId,
    pub lamprey_room_id: RoomId,
    pub discord_guild_id: DcGuildId,
    pub discord_channel_id: DcChannelId,
    pub discord_thread_id: Option<DcChannelId>,
    pub discord_webhook: String,
}



struct PortalConfigRow {
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
            discord_thread_id: value
                .discord_thread_id
                .map(|v| v.parse())
                .transpose()?,
            discord_webhook: value.discord_webhook,
        })
    }
}

pub struct MessageMetadata {
    pub chat_id: MessageId,
    pub chat_thread_id: ThreadId,
    pub discord_id: DcMessageId,
    /// the THREAD id, falling back to channel id
    pub discord_channel_id: DcChannelId,
}

struct MessageMetadataRow {
    chat_id: String,
    chat_thread_id: String,
    discord_id: String,
    discord_channel_id: String,
}

pub struct AttachmentMetadata {
    pub chat_id: MediaId,
    pub discord_id: DcAttachmentId,
}

struct AttachmentMetadataRow {
    chat_id: String,
    discord_id: String,
}

#[derive(Debug)]
pub struct Puppet {
    pub id: Uuid,
    pub ext_platform: String,
    pub ext_id: String,
    pub ext_avatar: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    // TODO: remove Option
    pub bot: Option<bool>,
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

#[async_trait]
pub trait Data {
    async fn get_portals(&self) -> Result<Vec<PortalConfig>>;
    async fn get_portal_by_thread_id(&self, id: ThreadId) -> Result<Option<PortalConfig>>;
    async fn get_portal_by_discord_channel(
        &self,
        id: DcChannelId,
    ) -> Result<Option<PortalConfig>>;
    async fn get_message(&self, message_id: MessageId) -> Result<Option<MessageMetadata>>;
    async fn get_message_dc(&self, message_id: DcMessageId) -> Result<Option<MessageMetadata>>;
    async fn get_attachment(&self, media_id: MediaId) -> Result<Option<AttachmentMetadata>>;
    async fn get_attachment_dc(
        &self,
        attachment_id: DcAttachmentId,
    ) -> Result<Option<AttachmentMetadata>>;
    async fn get_last_message_ch(&self, thread_id: ThreadId) -> Result<Option<MessageMetadata>>;
    async fn insert_message(&self, meta: MessageMetadata) -> Result<()>;
    async fn insert_attachment(&self, meta: AttachmentMetadata) -> Result<()>;
    async fn delete_message(&self, message_id: MessageId) -> Result<()>;
    async fn delete_message_dc(&self, message_id: DcMessageId) -> Result<()>;
    async fn get_puppet(&self, ext_platform: &str, ext_id: &str) -> Result<Option<Puppet>>;
    async fn insert_puppet(&self, data: Puppet) -> Result<()>;
}

#[async_trait]
impl Data for Globals {
    async fn get_portals(&self) -> Result<Vec<PortalConfig>> {
        let rows = query_as!(PortalConfigRow, "SELECT * FROM portal")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn get_portal_by_thread_id(&self, id: ThreadId) -> Result<Option<PortalConfig>> {
        let id = id.to_string();
        let row = query_as!(
            PortalConfigRow,
            "SELECT * FROM portal WHERE lamprey_thread_id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.try_into()).transpose()
    }

    async fn get_portal_by_discord_channel(
        &self,
        id: DcChannelId,
    ) -> Result<Option<PortalConfig>> {
        let id = id.to_string();
        let row = query_as!(
            PortalConfigRow,
            "SELECT * FROM portal WHERE discord_channel_id = ? OR discord_thread_id = ?",
            id,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.try_into()).transpose()
    }

    async fn get_message(&self, message_id: MessageId) -> Result<Option<MessageMetadata>> {
        let b1 = message_id.to_string();
        let row = query_as!(
            MessageMetadataRow,
            "SELECT * FROM message WHERE chat_id = ?",
            b1
        )
        .fetch_optional(&self.pool)
        .await?;
        let meta = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };
        Ok(meta)
    }

    async fn get_message_dc(&self, message_id: DcMessageId) -> Result<Option<MessageMetadata>> {
        let b1 = message_id.to_string();
        let row = query_as!(
            MessageMetadataRow,
            "SELECT * FROM message WHERE discord_id = ?",
            b1
        )
        .fetch_optional(&self.pool)
        .await?;
        let meta = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };
        Ok(meta)
    }

    async fn get_attachment_dc(
        &self,
        attachment_id: DcAttachmentId,
    ) -> Result<Option<AttachmentMetadata>> {
        let b1 = attachment_id.to_string();
        let row = query_as!(
            AttachmentMetadataRow,
            "SELECT * FROM attachment WHERE discord_id = ?",
            b1
        )
        .fetch_optional(&self.pool)
        .await?;
        let meta = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };
        Ok(meta)
    }

    async fn get_attachment(&self, media_id: MediaId) -> Result<Option<AttachmentMetadata>> {
        let b1 = media_id.to_string();
        let row = query_as!(
            AttachmentMetadataRow,
            "SELECT * FROM attachment WHERE chat_id = ?",
            b1
        )
        .fetch_optional(&self.pool)
        .await?;
        let meta = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };
        Ok(meta)
    }

    async fn insert_message(&self, meta: MessageMetadata) -> Result<()> {
        let row: MessageMetadataRow = meta.into();
        query!(
            "INSERT OR IGNORE INTO message (chat_id, chat_thread_id, discord_id, discord_channel_id) VALUES ($1, $2, $3, $4)",
            row.chat_id,
            row.chat_thread_id,
            row.discord_id,
            row.discord_channel_id,
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn insert_attachment(&self, meta: AttachmentMetadata) -> Result<()> {
        let row: AttachmentMetadataRow = meta.into();
        query!(
            "INSERT OR IGNORE INTO attachment (chat_id, discord_id) VALUES ($1, $2)",
            row.chat_id,
            row.discord_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_last_message_ch(&self, thread_id: ThreadId) -> Result<Option<MessageMetadata>> {
        let b1 = thread_id.to_string();
        let row = query_as!(
            MessageMetadataRow,
            "SELECT * FROM message WHERE chat_thread_id = ? ORDER BY chat_id DESC LIMIT 1",
            b1
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.try_into()).transpose()?)
    }

    async fn delete_message(&self, message_id: MessageId) -> Result<()> {
        let b1 = message_id.to_string();
        query!("DELETE FROM message WHERE chat_id = ?", b1)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_message_dc(&self, message_id: DcMessageId) -> Result<()> {
        let b1 = message_id.to_string();
        query!("DELETE FROM message WHERE discord_id = ?", b1)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_puppet(&self, ext_platform: &str, ext_id: &str) -> Result<Option<Puppet>> {
        let row = query_as!(
            Puppet,
            r#"
            SELECT id AS "id!: Uuid", ext_platform, ext_id, ext_avatar, name, avatar, bot
            FROM puppet WHERE ext_platform = ? AND ext_id = ?
            "#,
            ext_platform,
            ext_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn insert_puppet(&self, data: Puppet) -> Result<()> {
        query!(
            r#"
            INSERT OR REPLACE INTO puppet (id, ext_platform, ext_id, ext_avatar, name, avatar, bot)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            data.id,
            data.ext_platform,
            data.ext_id,
            data.ext_avatar,
            data.name,
            data.avatar,
            data.bot,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
