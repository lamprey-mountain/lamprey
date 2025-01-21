use async_trait::async_trait;
use sqlx::{query, query_as, Acquire};
use types::{Message, MessageId, PaginationDirection, PaginationQuery, PaginationResponse, SearchMessageRequest};
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::types::{DbMessage, DbMessageType, DbUser, User, UserCreate, UserId, UserPatch, UserVerId};

use crate::data::DataSearch;

use super::Postgres;

#[async_trait]
impl DataSearch for Postgres {
    async fn search_message(
        &self,
        user_id: UserId,
        query: SearchMessageRequest,
        paginate: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<MessageId> = paginate.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!( 
            DbMessage,
            r#"
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
            ),
            thread_viewer as (
                select thread.id from thread
                join room_member on thread.room_id = room_member.room_id
                where room_member.user_id = $1
            )
        select
            msg.type as "message_type: DbMessageType",
            msg.id,
            msg.thread_id, 
            msg.version_id,
            msg.ordering,
            msg.content,
            msg.metadata,
            msg.reply_id,
            msg.override_name,
            row_to_json(usr) as "author!: serde_json::Value",
            coalesce(att_json.attachments, '[]'::json) as "attachments!: serde_json::Value",
            false as "is_pinned!"
        from message_coalesced as msg
        join usr on usr.id = msg.author_id
        left join att_json on att_json.version_id = msg.version_id
        join thread_viewer on msg.thread_id = thread_viewer.id
        where msg.deleted_at is null
          and msg.id > $2 AND msg.id < $3
          and content @@ websearch_to_tsquery($6)
		order by (CASE WHEN $4 = 'f' THEN msg.id END), msg.id DESC LIMIT $5
            "#,
            user_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32,
            query.query,
        )
        .fetch_all(&mut *tx)
        .await?;
    // TODO: get (approx?) total?
    tx.rollback().await?;
    let has_more = items.len() > p.limit as usize;
    let mut items: Vec<_> = items
        .into_iter()
        .take(p.limit as usize)
        .map(Into::into)
        .collect();
    if p.dir == PaginationDirection::B {
        items.reverse();
    }
    Ok(PaginationResponse {
        items,
        total: 0,
        has_more,
    })
    }
}
