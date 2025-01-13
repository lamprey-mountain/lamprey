use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataMessage for Postgres {
    async fn message_create(&self, create: MessageCreate) -> Result<MessageId> {
        let mut conn = self.pool.acquire().await?;
        let message_id = Uuid::now_v7();
        let atts: Vec<Uuid> = create.attachment_ids.iter().map(|i| i.into_inner()).collect();
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, attachments)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, $10)
        "#, message_id, create.thread_id.into_inner(), message_id, create.content, create.metadata, create.reply_id.map(|i| i.into_inner()), create.author_id.into_inner(), create.message_type as _, create.override_name, &atts)
        .execute(&mut *conn)
        .await?;
        Ok(message_id.into())
    }

    async fn message_update(&self, message_id: MessageId, create: MessageCreate) -> Result<MessageVerId> {
        todo!()
    }

    async fn message_get(&self, thread_id: ThreadId, id: MessageId) -> Result<Message> {
        let mut conn = self.pool.acquire().await?;
        let row = query!(r#"
            with
            att_unnest as (select version_id, unnest(attachments) as media_id from message),
            att_json as (
                select version_id, json_agg(row_to_json(media)) as attachments
                from att_unnest
                join media on att_unnest.media_id = media.id
                group by att_unnest.version_id
            ),
            message_coalesced as (
                select *
                from (select *, row_number() over(partition by id order by version_id desc) as row_num
                    from message)
                where row_num = 1
            )
            SELECT
                msg.type as "message_type: MessageType",
                msg.id,
                msg.thread_id, 
                msg.version_id,
                msg.ordering,
                msg.content,
                msg.metadata,
                msg.reply_id,
                msg.override_name,
                row_to_json(usr) as "author!",
                coalesce(att_json.attachments, '[]'::json) as "attachments!"
            FROM message_coalesced AS msg
            JOIN usr ON usr.id = msg.author_id
            left JOIN att_json ON att_json.version_id = msg.version_id
                 WHERE thread_id = $1 AND msg.id = $2 AND msg.deleted_at IS NULL
        "#, thread_id.into_inner(), id.into_inner()).fetch_one(&mut *conn).await?;
        let msg = Message {
            message_type: MessageType::Default,
            id,
            thread_id,
            version_id: MessageVerId(row.version_id),
            nonce: None,
            ordering: row.ordering,
            content: row.content,
            attachments: serde_json::from_value(row.attachments).expect("invalid data in database!"),
            metadata: row.metadata,
            reply_id: row.reply_id.map(MessageId),
            override_name: row.override_name,
            author: serde_json::from_value(row.author).expect("invalid data in database!"),
            is_pinned: false,
        };
        Ok(msg)
    }

    async fn message_list(
            &self,
            thread_id: ThreadId,
            pagination: PaginationQuery<MessageId>,
        ) -> Result<PaginationResponse<Message>> {
        todo!()
    }

    async fn message_delete(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        todo!()
    }

    async fn message_version_get(&self, thread_id: ThreadId, message_id: MessageId, version_id: MessageVerId) -> Result<Message> {
        todo!()
    }

    async fn message_version_delete(&self, thread_id: ThreadId, message_id: MessageId, version_id: MessageVerId) -> Result<()> {
        todo!()
    }

    async fn message_version_list(&self, thread_id: ThreadId, message_id: MessageId, pagination: PaginationQuery<MessageVerId>) -> Result<PaginationResponse<Message>> {
        todo!()
    }
}
