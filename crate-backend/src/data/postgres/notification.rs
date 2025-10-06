use async_trait::async_trait;
use common::v1::types::{
    notifications::{
        InboxListParams, InboxThreadsParams, Notification, NotificationFlush, NotificationMarkRead,
        NotificationReason,
    },
    NotificationId, PaginationDirection, PaginationQuery, PaginationResponse, Thread,
};
use sqlx::{query, query_file_as, query_file_scalar, query_scalar, Acquire};
use uuid::Uuid;

use crate::{
    data::DataNotification,
    error::Result,
    gen_paginate,
    types::{DbNotification, DbThread, DbThreadType, ThreadId, UserId},
};

use super::Postgres;

fn notif_reason_str(r: NotificationReason) -> &'static str {
    match r {
        NotificationReason::Mention => "Mention",
        NotificationReason::MentionBulk => "MentionBulk",
        NotificationReason::Reply => "Reply",
        NotificationReason::Reminder => "Reminder",
    }
}

fn notif_reason_parse(s: &str) -> NotificationReason {
    match s {
        "Mention" => NotificationReason::Mention,
        "MentionBulk" => NotificationReason::MentionBulk,
        "Reply" => NotificationReason::Reply,
        "Reminder" => NotificationReason::Reminder,
        _ => panic!("invalid data in db"),
    }
}

impl From<DbNotification> for Notification {
    fn from(val: DbNotification) -> Self {
        Notification {
            id: val.id.into(),
            thread_id: val.thread_id.into(),
            message_id: val.message_id.into(),
            reason: notif_reason_parse(&val.reason),
            added_at: val.added_at.into(),
        }
    }
}

#[async_trait]
impl DataNotification for Postgres {
    async fn notification_add(&self, user_id: UserId, notif: Notification) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let room_id: Option<Uuid> = query_scalar!(
            "SELECT room_id FROM thread WHERE id = $1",
            notif.thread_id.into_inner()
        )
        .fetch_one(&mut *conn)
        .await?;

        let added_at = time::PrimitiveDateTime::new(notif.added_at.date(), notif.added_at.time());
        query!(
            "INSERT INTO inbox (id, user_id, room_id, thread_id, message_id, reason, added_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            notif.id.into_inner(),
            user_id.into_inner(),
            room_id,
            notif.thread_id.into_inner(),
            notif.message_id.into_inner(),
            notif_reason_str(notif.reason),
            added_at,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    async fn notification_delete(&self, user_id: UserId, notif_id: NotificationId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            "DELETE FROM inbox WHERE id = $1 AND user_id = $2",
            notif_id.into_inner(),
            user_id.into_inner()
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    async fn notification_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<NotificationId>,
        params: InboxListParams,
    ) -> Result<PaginationResponse<Notification>> {
        let p: super::Pagination<_> = pagination.try_into()?;

        let room_ids: Vec<Uuid> = params.room_id.iter().map(|id| id.into_inner()).collect();
        let thread_ids: Vec<Uuid> = params.thread_id.iter().map(|id| id.into_inner()).collect();

        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbNotification,
                "sql/notification_list.sql",
                user_id.into_inner(),
                params.include_read,
                &room_ids,
                &thread_ids,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
            ),
            query_file_scalar!(
                "sql/notification_list_count.sql",
                user_id.into_inner(),
                params.include_read,
                &room_ids,
                &thread_ids,
            ),
            |i: &Notification| i.id.to_string()
        )
    }

    async fn notification_mark_read(
        &self,
        user_id: UserId,
        params: NotificationMarkRead,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        if params.everything {
            query!(
                "UPDATE inbox SET read_at = now() WHERE user_id = $1",
                user_id.into_inner()
            )
            .execute(&mut *conn)
            .await?;
        } else if !params.message_ids.is_empty()
            || !params.thread_ids.is_empty()
            || !params.room_ids.is_empty()
        {
            let message_ids: Vec<Uuid> = params
                .message_ids
                .iter()
                .map(|id| id.into_inner())
                .collect();
            let thread_ids: Vec<Uuid> =
                params.thread_ids.iter().map(|id| id.into_inner()).collect();
            let room_ids: Vec<Uuid> = params.room_ids.iter().map(|id| id.into_inner()).collect();

            query!(
                "UPDATE inbox SET read_at = now() WHERE user_id = $1 AND (
                    (array_length($2::uuid[], 1) IS NOT NULL AND message_id = ANY($2)) OR
                    (array_length($3::uuid[], 1) IS NOT NULL AND thread_id = ANY($3)) OR
                    (array_length($4::uuid[], 1) IS NOT NULL AND room_id = ANY($4))
                )",
                user_id.into_inner(),
                &message_ids,
                &thread_ids,
                &room_ids,
            )
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    async fn notification_mark_unread(
        &self,
        user_id: UserId,
        params: NotificationMarkRead,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        if params.everything {
            query!(
                "UPDATE inbox SET read_at = NULL WHERE user_id = $1",
                user_id.into_inner()
            )
            .execute(&mut *conn)
            .await?;
        } else if !params.message_ids.is_empty()
            || !params.thread_ids.is_empty()
            || !params.room_ids.is_empty()
        {
            let message_ids: Vec<Uuid> = params
                .message_ids
                .iter()
                .map(|id| id.into_inner())
                .collect();
            let thread_ids: Vec<Uuid> =
                params.thread_ids.iter().map(|id| id.into_inner()).collect();
            let room_ids: Vec<Uuid> = params.room_ids.iter().map(|id| id.into_inner()).collect();

            query!(
                "UPDATE inbox SET read_at = NULL WHERE user_id = $1 AND (
                    (array_length($2::uuid[], 1) IS NOT NULL AND message_id = ANY($2)) OR
                    (array_length($3::uuid[], 1) IS NOT NULL AND thread_id = ANY($3)) OR
                    (array_length($4::uuid[], 1) IS NOT NULL AND room_id = ANY($4))
                )",
                user_id.into_inner(),
                &message_ids,
                &thread_ids,
                &room_ids,
            )
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    async fn notification_flush(&self, user_id: UserId, params: NotificationFlush) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let message_ids: Option<Vec<Uuid>> = params
            .message_ids
            .map(|ids| ids.iter().map(|id| id.into_inner()).collect());
        let thread_ids: Option<Vec<Uuid>> = params
            .thread_ids
            .map(|ids| ids.iter().map(|id| id.into_inner()).collect());
        let room_ids: Option<Vec<Uuid>> = params
            .room_ids
            .map(|ids| ids.iter().map(|id| id.into_inner()).collect());

        query!(
            "DELETE FROM inbox WHERE user_id = $1
                AND ($2 OR read_at IS NOT NULL)
                AND ($3::uuid IS NULL OR id <= $3)
                AND ($4::uuid IS NULL OR id >= $4)
                AND ($5::uuid[] IS NULL OR message_id = ANY($5))
                AND ($6::uuid[] IS NULL OR thread_id = ANY($6))
                AND ($7::uuid[] IS NULL OR room_id = ANY($7))
            ",
            user_id.into_inner(),
            params.include_unread,
            params.before.map(|id| id.into_inner()),
            params.after.map(|id| id.into_inner()),
            message_ids.as_deref(),
            thread_ids.as_deref(),
            room_ids.as_deref(),
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    async fn notification_list_threads(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ThreadId>,
        _params: InboxThreadsParams,
        list_params: InboxListParams,
    ) -> Result<PaginationResponse<Thread>> {
        let p: super::Pagination<_> = pagination.try_into()?;

        let room_ids: Vec<Uuid> = list_params
            .room_id
            .iter()
            .map(|id| id.into_inner())
            .collect();
        let thread_ids: Vec<Uuid> = list_params
            .thread_id
            .iter()
            .map(|id| id.into_inner())
            .collect();

        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbThread,
                "sql/notification_list_threads.sql",
                user_id.into_inner(),
                list_params.include_read,
                &room_ids,
                &thread_ids,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
            ),
            query_file_scalar!(
                "sql/notification_list_threads_count.sql",
                user_id.into_inner(),
                list_params.include_read,
                &room_ids,
                &thread_ids,
            ),
            |i: &Thread| i.id.to_string()
        )
    }
}
