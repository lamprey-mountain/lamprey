use async_trait::async_trait;
use common::v1::types::{
    search::{SearchMessageRequest, SearchThreadsRequest},
    Message, MessageId, PaginationDirection, PaginationQuery, PaginationResponse, Thread, ThreadId,
    UserId,
};
use sqlx::{query_file_as, query_file_scalar, Acquire};
use uuid::Uuid;

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
        let room_ids: Vec<Uuid> = query.room_id.iter().map(|id| **id).collect();
        let thread_ids: Vec<Uuid> = query.thread_id.iter().map(|id| **id).collect();
        let user_ids: Vec<Uuid> = query.user_id.iter().map(|id| **id).collect();
        let mentions_users: Vec<Uuid> = query.mentions_users.iter().map(|id| **id).collect();
        let mentions_roles: Vec<Uuid> = query.mentions_roles.iter().map(|id| **id).collect();
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
                &room_ids,
                &thread_ids,
                &user_ids,
                query.has_attachment,
                query.has_image,
                query.has_audio,
                query.has_video,
                query.has_link,
                query.has_embed,
                query.pinned,
                &query.link_hostnames,
                &mentions_users,
                &mentions_roles,
                query.mentions_everyone_room,
                query.mentions_everyone_thread,
            ),
            query_file_scalar!(
                "sql/search_message_count.sql",
                *user_id,
                query.query,
                &room_ids,
                &thread_ids,
                &user_ids,
                query.has_attachment,
                query.has_image,
                query.has_audio,
                query.has_video,
                query.has_link,
                query.has_embed,
                query.pinned,
                &query.link_hostnames,
                &mentions_users,
                &mentions_roles,
                query.mentions_everyone_room,
                query.mentions_everyone_thread,
            ),
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
        let room_ids: Vec<Uuid> = query.room_id.iter().map(|id| **id).collect();
        let parent_ids: Vec<Uuid> = query.parent_id.iter().map(|id| **id).collect();
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
                &room_ids,
                &parent_ids,
                query.archived,
                query.removed,
            ),
            query_file_scalar!(
                "sql/search_thread_count.sql",
                *user_id,
                query.query,
                &room_ids,
                &parent_ids,
                query.archived,
                query.removed,
            ),
            |i: &Thread| i.id.to_string()
        )
    }
}
