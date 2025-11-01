use async_trait::async_trait;
use common::v1::types::{ChannelId, PaginationQuery, PaginationResponse, UserId};
use sqlx::{query, query_file_as, query_file_scalar, Acquire};

use crate::data::postgres::Pagination;
use crate::data::DataThread;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::{Channel, ChannelVerId, DbChannel, PaginationDirection};

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

    async fn thread_auto_archive(&self) -> Result<Vec<ChannelId>> {
        let archived_threads = query!(
            r#"
            UPDATE channel 
            SET 
                version_id = $1,
                archived_at = NOW()
            WHERE 
                archived_at IS NULL
                AND deleted_at IS NULL
                AND type IN ('ThreadPublic', 'ThreadPrivate')
                AND auto_archive_duration IS NOT NULL
                AND last_activity_at IS NOT NULL
                AND last_activity_at + (auto_archive_duration * INTERVAL '1 second') < NOW()
            RETURNING id
            "#,
            ChannelVerId::new().into_inner()
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| ChannelId::from(row.id))
        .collect();

        Ok(archived_threads)
    }
}
