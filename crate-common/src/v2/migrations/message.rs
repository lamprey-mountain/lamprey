use crate::v1::types::message::Message as MessageV1;
use crate::v2::types::message::{Message as MessageV2, MessageVersion as MessageVersionV2};

// somewhat lossy message conversions

impl Into<MessageV1> for MessageV2 {
    fn into(self) -> MessageV1 {
        MessageV1 {
            message_type: self.latest_version.message_type,
            id: self.id,
            channel_id: self.channel_id,
            version_id: self.latest_version.version_id,
            nonce: None,
            author_id: self.author_id,
            mentions: self.latest_version.mentions,
            pinned: self.pinned,
            reactions: self.reactions,
            created_at: Some(self.created_at),
            deleted_at: self.deleted_at,
            removed_at: self.removed_at,
            edited_at: if *self.latest_version.version_id != *self.id {
                Some(self.latest_version.created_at)
            } else {
                None
            },
            thread: self.thread,
        }
    }
}

impl Into<MessageV2> for MessageV1 {
    fn into(self) -> MessageV2 {
        MessageV2 {
            id: self.id,
            channel_id: self.channel_id,
            latest_version: MessageVersionV2 {
                version_id: self.version_id,
                author_id: Some(self.author_id),
                message_type: self.message_type,
                mentions: self.mentions,
                created_at: self
                    .edited_at
                    .or(self.created_at)
                    .unwrap_or_else(|| self.version_id.try_into().unwrap()),
                deleted_at: self.deleted_at,
            },
            pinned: self.pinned,
            reactions: self.reactions,
            deleted_at: self.deleted_at,
            removed_at: self.removed_at,
            created_at: self
                .created_at
                .unwrap_or_else(|| self.id.try_into().unwrap()),
            author_id: self.author_id,
            thread: self.thread,
        }
    }
}
