use anyhow::Result;
use async_trait::async_trait;
use sqlx::{query, query_as};
use types::{MediaId, MessageId, ThreadId};

use crate::{common::Globals, util::{MatrixEventId, MatrixMediaUri, MatrixRoomId}};

pub struct MessageMetadata {
    pub chat_id: MessageId,
    pub chat_thread_id: ThreadId,
    pub matrix_id: MatrixEventId,
    pub matrix_room_id: MatrixRoomId,
}

struct MessageMetadataRow {
    chat_id: String,
    chat_thread_id: String,
    matrix_id: String,
    matrix_room_id: String,
}

pub struct AttachmentMetadata {
    pub chat_id: MediaId,
    pub matrix_id: MatrixMediaUri,
}

struct AttachmentMetadataRow {
    chat_id: String,
    matrix_id: String,
}

impl TryFrom<MessageMetadataRow> for MessageMetadata {
    type Error = anyhow::Error;

    fn try_from(row: MessageMetadataRow) -> Result<Self> {
        Ok(MessageMetadata {
            chat_id: row.chat_id.parse()?,
            chat_thread_id: row.chat_thread_id.parse()?,
            matrix_id: row.matrix_id.parse()?,
            matrix_room_id: row.matrix_room_id.parse()?,
        })
    }
}

impl From<MessageMetadata> for MessageMetadataRow {
    fn from(value: MessageMetadata) -> Self {
        MessageMetadataRow {
            chat_id: value.chat_id.to_string(),
            chat_thread_id: value.chat_thread_id.to_string(),
            matrix_id: value.matrix_id.to_string(),
            matrix_room_id: value.matrix_room_id.to_string(),
        }
    }
}

impl TryFrom<AttachmentMetadataRow> for AttachmentMetadata {
    type Error = anyhow::Error;

    fn try_from(row: AttachmentMetadataRow) -> Result<Self> {
        Ok(Self {
            chat_id: row.chat_id.parse()?,
            matrix_id: serde_json::from_str(&row.matrix_id)?,
        })
    }
}

impl From<AttachmentMetadata> for AttachmentMetadataRow {
    fn from(value: AttachmentMetadata) -> Self {
        Self {
            chat_id: value.chat_id.to_string(),
            matrix_id: value.matrix_id.to_string(),
        }
    }
}

#[async_trait]
pub trait Data {
    async fn get_message(&self, message_id: MessageId) -> Result<Option<MessageMetadata>>;
    async fn get_message_mx(&self, event_id: &MatrixEventId) -> Result<Option<MessageMetadata>>;
    async fn get_attachment(&self, media_id: MediaId) -> Result<Option<AttachmentMetadata>>;
    async fn get_attachment_mx(
        &self,
        media_id: MatrixMediaUri,
    ) -> Result<Option<AttachmentMetadata>>;
    async fn get_last_message_ch(&self, thread_id: ThreadId) -> Result<Option<MessageMetadata>>;
    async fn insert_message(&self, meta: MessageMetadata) -> Result<()>;
    async fn insert_attachment(&self, meta: AttachmentMetadata) -> Result<()>;
}

#[async_trait]
impl Data for Globals {
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

    async fn get_message_mx(&self, message_id: &MatrixEventId) -> Result<Option<MessageMetadata>> {
        let b1 = message_id.to_string();
        let row = query_as!(
            MessageMetadataRow,
            "SELECT * FROM message WHERE matrix_id = ?",
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

    async fn get_attachment_mx(
        &self,
        media_id: MatrixMediaUri,
    ) -> Result<Option<AttachmentMetadata>> {
        let b1 = media_id.to_string();
        let row = query_as!(
            AttachmentMetadataRow,
            "SELECT * FROM attachment WHERE matrix_id = ?",
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
            "INSERT INTO message (chat_id, chat_thread_id, matrix_id, matrix_room_id) VALUES ($1, $2, $3, $4)",
            row.chat_id,
            row.chat_thread_id,
            row.matrix_id,
            row.matrix_room_id,
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn insert_attachment(&self, meta: AttachmentMetadata) -> Result<()> {
        let row: AttachmentMetadataRow = meta.into();
        query!(
            "INSERT INTO attachment (chat_id, matrix_id) VALUES ($1, $2)",
            row.chat_id,
            row.matrix_id,
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
}
