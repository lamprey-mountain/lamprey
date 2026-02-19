use anyhow::Result;
use async_trait::async_trait;
use common::v1::types::{ChannelId, MediaId, MessageId, RoomId};
use serenity::all::{
    AttachmentId as DcAttachmentId, ChannelId as DcChannelId, MessageId as DcMessageId,
};
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::bridge_common::{Globals, PortalConfig, RealmConfig};
use crate::db::models::{
    AttachmentMetadata, AttachmentMetadataRow, MessageMetadata, MessageMetadataRow,
    PortalConfigRow, Puppet, RealmConfigRow,
};

#[async_trait]
pub trait Data {
    async fn get_portals(&self) -> Result<Vec<PortalConfig>>;
    async fn get_portal_by_thread_id(&self, id: ChannelId) -> Result<Option<PortalConfig>>;
    async fn get_portal_by_discord_channel(&self, id: DcChannelId) -> Result<Option<PortalConfig>>;
    async fn insert_portal(&self, portal: PortalConfig) -> Result<()>;
    async fn delete_portal(&self, lamprey_thread_id: ChannelId) -> Result<()>;
    async fn get_message(&self, message_id: MessageId) -> Result<Option<MessageMetadata>>;
    async fn get_message_dc(&self, message_id: DcMessageId) -> Result<Option<MessageMetadata>>;
    async fn get_attachment(&self, media_id: MediaId) -> Result<Option<AttachmentMetadata>>;
    async fn get_attachment_dc(
        &self,
        attachment_id: DcAttachmentId,
    ) -> Result<Option<AttachmentMetadata>>;
    async fn get_last_message_ch(&self, thread_id: ChannelId) -> Result<Option<MessageMetadata>>;
    async fn get_last_message_dc(&self, channel_id: DcChannelId)
        -> Result<Option<MessageMetadata>>;
    async fn insert_message(&self, meta: MessageMetadata) -> Result<()>;
    async fn insert_attachment(&self, meta: AttachmentMetadata) -> Result<()>;
    async fn delete_message(&self, message_id: MessageId) -> Result<()>;
    async fn delete_message_dc(&self, message_id: DcMessageId) -> Result<()>;
    async fn get_puppet(&self, ext_platform: &str, ext_id: &str) -> Result<Option<Puppet>>;
    async fn insert_puppet(&self, data: Puppet) -> Result<()>;
    async fn get_realms(&self) -> Result<Vec<RealmConfig>>;
    async fn insert_realm(&self, config: RealmConfig) -> Result<()>;
    async fn delete_realm(&self, lamprey_room_id: RoomId) -> Result<()>;
}

#[async_trait]
impl Data for Globals {
    async fn get_portals(&self) -> Result<Vec<PortalConfig>> {
        let rows = query_as!(PortalConfigRow, "SELECT * FROM portal")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn get_portal_by_thread_id(&self, id: ChannelId) -> Result<Option<PortalConfig>> {
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

    async fn get_portal_by_discord_channel(&self, id: DcChannelId) -> Result<Option<PortalConfig>> {
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

    async fn insert_portal(&self, portal: PortalConfig) -> Result<()> {
        let row: PortalConfigRow = portal.into();
        query!(
            r#"
            INSERT OR IGNORE INTO portal
            (lamprey_thread_id, lamprey_room_id, discord_guild_id, discord_channel_id, discord_thread_id, discord_webhook)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            row.lamprey_thread_id,
            row.lamprey_room_id,
            row.discord_guild_id,
            row.discord_channel_id,
            row.discord_thread_id,
            row.discord_webhook
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_portal(&self, lamprey_thread_id: ChannelId) -> Result<()> {
        let id = lamprey_thread_id.to_string();
        query!("DELETE FROM portal WHERE lamprey_thread_id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
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
            "INSERT OR IGNORE INTO message (chat_id, chat_thread_id, discord_id, discord_channel_id) VALUES (?, ?, ?, ?)",
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

    async fn get_last_message_ch(&self, thread_id: ChannelId) -> Result<Option<MessageMetadata>> {
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

    async fn get_last_message_dc(
        &self,
        channel_id: DcChannelId,
    ) -> Result<Option<MessageMetadata>> {
        let id = channel_id.to_string();
        let row = query_as!(
            MessageMetadataRow,
            "SELECT * FROM message WHERE discord_channel_id = ? ORDER BY discord_id DESC LIMIT 1",
            id
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
            SELECT id AS "id!: Uuid", ext_platform, ext_id, ext_avatar, ext_banner, name, avatar, banner, bot
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
            INSERT OR REPLACE INTO puppet (id, ext_platform, ext_id, ext_avatar, ext_banner, name, avatar, banner, bot)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            data.id,
            data.ext_platform,
            data.ext_id,
            data.ext_avatar,
            data.ext_banner,
            data.name,
            data.avatar,
            data.banner,
            data.bot,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_realms(&self) -> Result<Vec<RealmConfig>> {
        let rows = query_as!(
            RealmConfigRow,
            r#"
            SELECT
              lamprey_room_id AS "lamprey_room_id!: String",
              discord_guild_id AS "discord_guild_id!: String",
              continuous
            FROM realm
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn insert_realm(&self, config: RealmConfig) -> Result<()> {
        let row: RealmConfigRow = config.into();
        query!(
            r#"
             INSERT OR REPLACE INTO realm (lamprey_room_id, discord_guild_id, continuous)
             VALUES (?, ?, ?)
             "#,
            row.lamprey_room_id,
            row.discord_guild_id,
            row.continuous
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_realm(&self, lamprey_room_id: RoomId) -> Result<()> {
        let id = lamprey_room_id.to_string();
        query!("DELETE FROM realm WHERE lamprey_room_id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
