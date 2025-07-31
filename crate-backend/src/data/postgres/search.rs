use async_trait::async_trait;
use common::v1::types::search::SearchMessageRequest;
use common::v1::types::{
    Message, MessageId, PaginationDirection, PaginationQuery, PaginationResponse,
};
use sqlx::{query_file_as, query_file_scalar, Acquire};

use crate::data::postgres::message::{DbMessage, DbMessageType};
use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::gen_paginate;
use common::v1::types::UserId;

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
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                "sql/search_message.sql",
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                query.query,
            ),
            query_file_scalar!(
                "sql/search_message_count.sql",
                user_id.into_inner(),
                query.query,
            ),
            |i: &Message| i.id.to_string()
        )
    }
}
