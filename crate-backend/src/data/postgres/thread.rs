use async_trait::async_trait;
use common::v1::types::{ChannelId, PaginationQuery, PaginationResponse, UserId};
use sqlx::{query_file_as, query_file_scalar, Acquire};

use crate::data::postgres::Pagination;
use crate::data::DataThread;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::{Channel, DbChannel, PaginationDirection, RoomId};

use super::Postgres;

#[async_trait]
impl DataThread for Postgres {
    async fn thread_list_active(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/thread_list_active.sql",
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *parent_id,
                *user_id,
                include_all
            ),
            query_file_scalar!(
                "sql/thread_list_active_count.sql",
                *parent_id,
                *user_id,
                include_all
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn thread_list_archived(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/thread_list_archived.sql",
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *parent_id,
                *user_id,
                include_all
            ),
            query_file_scalar!(
                "sql/thread_list_archived_count.sql",
                *parent_id,
                *user_id,
                include_all
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn thread_list_removed(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/thread_list_removed.sql",
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *parent_id,
                *user_id,
                include_all
            ),
            query_file_scalar!(
                "sql/thread_list_removed_count.sql",
                *parent_id,
                *user_id,
                include_all
            ),
            |i: &Channel| i.id.to_string()
        )
    }
}
