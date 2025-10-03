use async_trait::async_trait;
use common::v1::types::{
    search::{SearchMessageRequest, SearchThreadsRequest},
    Message, MessageId, PaginationDirection, PaginationQuery, PaginationResponse, Thread, ThreadId,
    UserId,
};
use sqlx::{query_file_as, query_file_scalar, Acquire};

use crate::{
    data::{
        postgres::{
            message::{DbMessage, DbMessageType},
            Pagination,
        },
        DataSearch,
    },
    error::Result,
    gen_paginate,
    types::{DbThread, DbThreadType},
};

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
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                "sql/search_message.sql",
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                query.query,
            ),
            query_file_scalar!("sql/search_message_count.sql", *user_id, query.query,),
            |i: &Message| i.id.to_string()
        )
    }

    async fn search_thread(
        &self,
        user_id: UserId,
        query: SearchThreadsRequest,
        paginate: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>> {
        let p: Pagination<ThreadId> = paginate.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbThread,
                "sql/search_thread.sql",
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                query.query,
            ),
            query_file_scalar!("sql/search_thread_count.sql", *user_id, query.query,),
            |i: &Thread| i.id.to_string()
        )
    }
}
