use async_trait::async_trait;
use common::v1::types::{
    calendar::{CalendarEvent, CalendarEventCreate, CalendarEventPatch},
    pagination::{PaginationDirection, PaginationQuery, PaginationResponse},
    CalendarEventId, ChannelId, UserId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{
    data::{postgres::Pagination, DataCalendar},
    error::Result,
    gen_paginate,
};

use super::Postgres;

pub struct DbCalendarEvent {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub timezone: Option<String>,
    pub recurrence: Option<serde_json::Value>,
    pub start_at: PrimitiveDateTime,
    pub end_at: PrimitiveDateTime,
}

impl From<DbCalendarEvent> for CalendarEvent {
    fn from(val: DbCalendarEvent) -> Self {
        Self {
            id: val.id.into(),
            channel_id: val.channel_id.into(),
            creator_id: val.creator_id.map(|i| i.into()),
            title: val.title,
            description: val.description,
            location: val.location,
            url: val.url.and_then(|u| u.parse().ok()),
            timezone: val.timezone,
            recurrence: val
                .recurrence
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            start: val.start_at.into(),
            end: val.end_at.into(),
        }
    }
}

#[async_trait]
impl DataCalendar for Postgres {
    async fn calendar_event_create(
        &self,
        create: CalendarEventCreate,
        channel_id: ChannelId,
        creator_id: UserId,
    ) -> Result<CalendarEvent> {
        let event_id = CalendarEventId::new();
        let recurrence = if create.recurrence.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&create.recurrence)?)
        };
        let event = query_as!(
            DbCalendarEvent,
            r#"
            INSERT INTO calendar_event (id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
            "#,
            *event_id,
            *channel_id,
            *creator_id,
            create.title,
            create.description,
            create.location,
            create.url.as_ref().map(|u| u.as_str()),
            create.timezone,
            recurrence,
            PrimitiveDateTime::from(create.start),
            PrimitiveDateTime::from(create.end),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(event.into())
    }

    async fn calendar_event_get(&self, event_id: CalendarEventId) -> Result<CalendarEvent> {
        let event = query_as!(
            DbCalendarEvent,
            r#"
            SELECT id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
            FROM calendar_event
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            *event_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(event.into())
    }

    async fn calendar_event_list(
        &self,
        channel_id: ChannelId,
        pagination: PaginationQuery<CalendarEventId>,
    ) -> Result<PaginationResponse<CalendarEvent>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbCalendarEvent,
                r#"
                SELECT id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
                FROM calendar_event
                WHERE channel_id = $1 AND id > $2 AND id < $3 AND deleted_at IS NULL
                ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
                "#,
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM calendar_event WHERE channel_id = $1 AND deleted_at IS NULL",
                *channel_id
            ),
            |i: &CalendarEvent| i.id.to_string()
        )
    }

    async fn calendar_event_update(
        &self,
        event_id: CalendarEventId,
        patch: CalendarEventPatch,
    ) -> Result<CalendarEvent> {
        let mut tx = self.pool.begin().await?;
        let event = query_as!(
            DbCalendarEvent,
            "SELECT id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at FROM calendar_event WHERE id = $1 FOR UPDATE",
            *event_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let title = patch.title.unwrap_or(event.title);
        let description = patch.description.unwrap_or(event.description);
        let location = patch.location.unwrap_or(event.location);
        let url = patch
            .url
            .map(|u| u.map(|u| u.to_string()))
            .unwrap_or(event.url);
        let channel_id = patch.channel_id.map(|c| *c).unwrap_or(event.channel_id);

        let updated_event = query_as!(
            DbCalendarEvent,
            r#"
            UPDATE calendar_event
            SET title = $2, description = $3, location = $4, url = $5, channel_id = $6, updated_at = now()
            WHERE id = $1
            RETURNING id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
            "#,
            *event_id,
            title,
            description,
            location,
            url,
            channel_id,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(updated_event.into())
    }

    async fn calendar_event_delete(&self, event_id: CalendarEventId) -> Result<()> {
        query!(
            "UPDATE calendar_event SET deleted_at = now() WHERE id = $1",
            *event_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn calendar_event_rsvp_put(
        &self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()> {
        query!(
            "INSERT INTO calendar_event_rsvp (event_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            *event_id,
            *user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn calendar_event_rsvp_delete(
        &self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()> {
        query!(
            "DELETE FROM calendar_event_rsvp WHERE event_id = $1 AND user_id = $2",
            *event_id,
            *user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn calendar_event_rsvp_list(&self, event_id: CalendarEventId) -> Result<Vec<UserId>> {
        let user_ids = query_scalar!(
            "SELECT user_id FROM calendar_event_rsvp WHERE event_id = $1",
            *event_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(user_ids.into_iter().map(Into::into).collect())
    }
}
