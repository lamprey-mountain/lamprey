use async_trait::async_trait;
use common::v1::types::notifications::{
    InboxListParams, Notification, NotificationFlush, NotificationMarkRead, NotificationType,
};
use common::v1::types::{NotificationId, PaginationDirection, PaginationQuery, PaginationResponse};
use sqlx::{query, query_file, query_file_as, query_file_scalar, Acquire};
use uuid::Uuid;

use crate::{
    data::DataNotification,
    error::Result,
    gen_paginate,
    types::{DbNotification, DbNotificationType, RoomId, UserId},
};

use super::Postgres;

impl From<DbNotification> for Notification {
    fn from(val: DbNotification) -> Self {
        let ty = match val.ty {
            DbNotificationType::Message => NotificationType::Message {
                room_id: val
                    .room_id
                    .map(|id| id.into())
                    .unwrap_or_else(|| RoomId::new()),
                channel_id: val.channel_id.into(),
                message_id: val.message_id.into(),
            },
            DbNotificationType::Reaction => NotificationType::Reaction {
                room_id: val
                    .room_id
                    .map(|id| id.into())
                    .unwrap_or_else(|| RoomId::new()),
                channel_id: val.channel_id.into(),
                message_id: val.message_id.into(),
            },
        };
        Notification {
            id: val.id.into(),
            ty,
            added_at: val.added_at.into(),
            read_at: val.read_at.map(|t| t.into()),
            note: None,
        }
    }
}

#[async_trait]
impl DataNotification for Postgres {
    async fn notification_add(&self, user_id: UserId, notif: Notification) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let (room_id, channel_id, message_id, ty) = match &notif.ty {
            NotificationType::Message {
                room_id,
                channel_id,
                message_id,
            } => (
                Some(room_id.into_inner()),
                channel_id.into_inner(),
                message_id.into_inner(),
                DbNotificationType::Message,
            ),
            NotificationType::Reaction {
                room_id,
                channel_id,
                message_id,
            } => (
                Some(room_id.into_inner()),
                channel_id.into_inner(),
                message_id.into_inner(),
                DbNotificationType::Reaction,
            ),
        };

        let added_at = time::PrimitiveDateTime::new(notif.added_at.date(), notif.added_at.time());
        query!(
            "INSERT INTO inbox (id, user_id, room_id, channel_id, message_id, type, added_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            notif.id.into_inner(),
            user_id.into_inner(),
            room_id,
            channel_id,
            message_id,
            ty as _,
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
        let channel_ids: Vec<Uuid> = params.channel_id.iter().map(|id| id.into_inner()).collect();

        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbNotification,
                "sql/notification_list.sql",
                user_id.into_inner(),
                params.include_read,
                &room_ids,
                &channel_ids,
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
                &channel_ids,
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
            || !params.channel_ids.is_empty()
            || !params.room_ids.is_empty()
        {
            let message_ids: Vec<Uuid> = params
                .message_ids
                .iter()
                .map(|id| id.into_inner())
                .collect();
            let channel_ids: Vec<Uuid> = params
                .channel_ids
                .iter()
                .map(|id| id.into_inner())
                .collect();
            let room_ids: Vec<Uuid> = params.room_ids.iter().map(|id| id.into_inner()).collect();

            query!(
                "UPDATE inbox SET read_at = now() WHERE user_id = $1 AND (
                    (array_length($2::uuid[], 1) IS NOT NULL AND message_id = ANY($2)) OR
                    (array_length($3::uuid[], 1) IS NOT NULL AND channel_id = ANY($3)) OR
                    (array_length($4::uuid[], 1) IS NOT NULL AND room_id = ANY($4))
                )",
                user_id.into_inner(),
                &message_ids,
                &channel_ids,
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
            || !params.channel_ids.is_empty()
            || !params.room_ids.is_empty()
        {
            let message_ids: Vec<Uuid> = params
                .message_ids
                .iter()
                .map(|id| id.into_inner())
                .collect();
            let channel_ids: Vec<Uuid> = params
                .channel_ids
                .iter()
                .map(|id| id.into_inner())
                .collect();
            let room_ids: Vec<Uuid> = params.room_ids.iter().map(|id| id.into_inner()).collect();

            query!(
                "UPDATE inbox SET read_at = NULL WHERE user_id = $1 AND (
                    (array_length($2::uuid[], 1) IS NOT NULL AND message_id = ANY($2)) OR
                    (array_length($3::uuid[], 1) IS NOT NULL AND channel_id = ANY($3)) OR
                    (array_length($4::uuid[], 1) IS NOT NULL AND room_id = ANY($4))
                )",
                user_id.into_inner(),
                &message_ids,
                &channel_ids,
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
        let channel_ids: Option<Vec<Uuid>> = params
            .channel_ids
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
                AND ($6::uuid[] IS NULL OR channel_id = ANY($6))
                AND ($7::uuid[] IS NULL OR room_id = ANY($7))
            ",
            user_id.into_inner(),
            params.include_unread,
            params.before.map(|id| id.into_inner()),
            params.after.map(|id| id.into_inner()),
            message_ids.as_deref(),
            channel_ids.as_deref(),
            room_ids.as_deref(),
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    async fn notification_get_unpushed(&self, limit: u32) -> Result<Vec<(UserId, Notification)>> {
        let rows = query_file!("sql/notification_get_unpushed.sql", limit as i32)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let user_id = UserId::from(r.user_id);
                let notif: Notification = DbNotification {
                    id: r.id,
                    room_id: r.room_id,
                    channel_id: r.channel_id,
                    message_id: r.message_id,
                    ty: r.ty,
                    added_at: r.added_at,
                    read_at: r.read_at,
                }
                .into();
                (user_id, notif)
            })
            .collect())
    }

    async fn notification_set_pushed(&self, ids: &[NotificationId]) -> Result<()> {
        let ids: Vec<Uuid> = ids.iter().map(|id| id.into_inner()).collect();
        query!(
            "UPDATE inbox SET pushed_at = now() WHERE id = ANY($1)",
            &ids
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
