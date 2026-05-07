use async_trait::async_trait;
use common::v1::types::{
    calendar::{
        CalendarEvent, CalendarEventCreate, CalendarEventListQuery, CalendarEventParticipant,
        CalendarEventParticipantQuery, CalendarEventPatch, CalendarOverwrite, CalendarOverwritePut,
        CalendarRsvpStatus, Timezone,
    },
    error::{ApiError, ErrorCode},
    pagination::{PaginationDirection, PaginationResponse},
    CalendarEventId, ChannelId, PaginationKey, UserId,
};
use lamprey_backend_core::Error;
use std::collections::HashSet;

use sqlx::{query, query_as, query_scalar};
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
    pub end_at: Option<PrimitiveDateTime>,
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
            timezone: val.timezone.map(Timezone),
            recurrence: val.recurrence.and_then(|v| serde_json::from_value(v).ok()),
            starts_at: val.start_at.into(),
            ends_at: val.end_at.map(|e| e.into()),
        }
    }
}

pub struct DbCalendarOverwrite {
    pub event_id: Uuid,
    pub seq: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub start_at: Option<PrimitiveDateTime>,
    pub end_at: Option<PrimitiveDateTime>,
    pub cancelled: bool,
}

impl From<DbCalendarOverwrite> for CalendarOverwrite {
    fn from(val: DbCalendarOverwrite) -> Self {
        Self {
            event_id: val.event_id.into(),
            seq: val.seq as u64,
            title: val.title,
            extra_description: val.description,
            location: val.location.map(Some),
            url: val.url.and_then(|u| u.parse().ok()).map(Some),
            starts_at: val.start_at.map(Into::into),
            ends_at: val.end_at.map(|e| Some(e.into())),
            cancelled: val.cancelled,
        }
    }
}

#[async_trait]
impl DataCalendar for Postgres {
    async fn calendar_event_create(
        &mut self,
        create: CalendarEventCreate,
        channel_id: ChannelId,
        creator_id: UserId,
    ) -> Result<CalendarEvent> {
        let mut conn = self.acquire().await?;
        let event_id = CalendarEventId::new();
        let recurrence = if let Some(rec) = create.recurrence {
            Some(serde_json::to_value(&rec)?)
        } else {
            None
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
            create.timezone.as_ref().map(|u| &u.0),
            recurrence,
            PrimitiveDateTime::from(create.starts_at),
            create.ends_at.map(PrimitiveDateTime::from)
        )
        .fetch_one(conn.ext())
        .await?;

        Ok(event.into())
    }

    async fn calendar_event_get(&mut self, event_id: CalendarEventId) -> Result<CalendarEvent> {
        let mut conn = self.acquire().await?;
        let event = query_as!(
            DbCalendarEvent,
            r#"
            SELECT id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
            FROM calendar_event
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            *event_id
        )
        .fetch_one(conn.ext())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownCalendarEvent,
            )),
            e => Error::Sqlx(e),
        })?;

        Ok(event.into())
    }

    async fn calendar_event_list(
        &mut self,
        channel_id: ChannelId,
        query: CalendarEventListQuery,
    ) -> Result<PaginationResponse<CalendarEvent>> {
        let dir = query.dir.unwrap_or_default();
        let after = match dir {
            PaginationDirection::F => query.from.clone(),
            _ => query.to.clone(),
        };
        let after = after.unwrap_or(PaginationKey::min());
        let before = match dir {
            PaginationDirection::F => query.to,
            _ => query.from,
        };
        let before = before.unwrap_or(PaginationKey::max());
        let p: Pagination<_> = Pagination {
            before,
            after,
            dir,
            limit: query.limit.unwrap_or(10),
        };

        let from_time = query
            .from_time
            .map(Into::into)
            .unwrap_or(PrimitiveDateTime::MIN);
        let to_time = query
            .to_time
            .map(Into::into)
            .unwrap_or(PrimitiveDateTime::MAX);

        gen_paginate!(
            p, self,
            query_as!(
                DbCalendarEvent,
                r#"
                SELECT id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
                FROM calendar_event
                WHERE channel_id = $1 AND id > $2 AND id < $3 AND deleted_at IS NULL AND end_at > $6 AND start_at < $7
                ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
                "#,
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                from_time,
                to_time,
            ),
            query_scalar!(
                "SELECT count(*) FROM calendar_event WHERE channel_id = $1 AND deleted_at IS NULL AND end_at > $2 AND start_at < $3",
                *channel_id,
                from_time,
                to_time
            ),
            |i: &CalendarEvent| i.id.to_string()
        )
    }

    async fn calendar_event_update(
        &mut self,
        event_id: CalendarEventId,
        patch: CalendarEventPatch,
    ) -> Result<CalendarEvent> {
        let mut tx = self.begin_tx().await?;
        let event = query_as!(
            DbCalendarEvent,
            "SELECT id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at FROM calendar_event WHERE id = $1 FOR UPDATE",
            *event_id
        )
        .fetch_one(tx.ext())
        .await?;

        let title = patch.title.unwrap_or(event.title);
        let description = patch.description.unwrap_or(event.description);
        let location = patch.location.unwrap_or(event.location);
        let url = patch
            .url
            .map(|u| u.map(|u| u.to_string()))
            .unwrap_or(event.url);

        let start_at = patch
            .starts_at
            .map(PrimitiveDateTime::from)
            .unwrap_or(event.start_at);
        let end_at = patch
            .ends_at
            .map(|e| e.map(PrimitiveDateTime::from))
            .unwrap_or(event.end_at);

        let updated_event = query_as!(
            DbCalendarEvent,
            r#"
            UPDATE calendar_event
            SET title = $2, description = $3, location = $4, url = $5, updated_at = now(), start_at = $6, end_at = $7
            WHERE id = $1
            RETURNING id, channel_id, creator_id, title, description, location, url, timezone, recurrence, start_at, end_at
            "#,
            *event_id,
            title,
            description,
            location,
            url,
            start_at,
            end_at
        )
        .fetch_one(tx.ext())
        .await?;

        tx.commit().await?;

        Ok(updated_event.into())
    }

    async fn calendar_event_delete(&mut self, event_id: CalendarEventId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "UPDATE calendar_event SET deleted_at = now() WHERE id = $1",
            *event_id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn calendar_event_rsvp_put(
        &mut self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "INSERT INTO calendar_event_rsvp (event_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            *event_id,
            *user_id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn calendar_event_rsvp_delete(
        &mut self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "DELETE FROM calendar_event_rsvp WHERE event_id = $1 AND user_id = $2",
            *event_id,
            *user_id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn calendar_event_rsvp_get(
        &mut self,
        event_id: CalendarEventId,
        user_id: UserId,
    ) -> Result<CalendarEventParticipant> {
        let mut conn = self.acquire().await?;
        let exists = query_scalar!(
            "SELECT 1 FROM calendar_event_rsvp WHERE event_id = $1 AND user_id = $2",
            *event_id,
            *user_id
        )
        .fetch_optional(conn.ext())
        .await?;

        if exists.is_some() {
            Ok(CalendarEventParticipant {
                user_id,
                status: CalendarRsvpStatus::Interested,
                user: None,
                member: None,
            })
        } else {
            Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownCalendarEvent,
            )))
        }
    }

    async fn calendar_event_rsvp_list(
        &mut self,
        event_id: CalendarEventId,
        _query: CalendarEventParticipantQuery,
    ) -> Result<Vec<CalendarEventParticipant>> {
        let mut conn = self.acquire().await?;
        let user_ids = query_scalar!(
            "SELECT user_id FROM calendar_event_rsvp WHERE event_id = $1",
            *event_id
        )
        .fetch_all(conn.ext())
        .await?;

        Ok(user_ids
            .into_iter()
            .map(|uid| CalendarEventParticipant {
                user_id: uid.into(),
                status: CalendarRsvpStatus::Interested,
                user: None,
                member: None,
            })
            .collect())
    }

    async fn calendar_overwrite_put(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        put: CalendarOverwritePut,
    ) -> Result<CalendarOverwrite> {
        let mut conn = self.acquire().await?;
        let overwrite = query_as!(
            DbCalendarOverwrite,
            r#"
            INSERT INTO calendar_overwrite (event_id, seq, title, description, start_at, end_at, cancelled)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (event_id, seq) DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                start_at = EXCLUDED.start_at,
                end_at = EXCLUDED.end_at,
                cancelled = EXCLUDED.cancelled
            RETURNING event_id, seq as "seq!", title, description, location, url, start_at, end_at, cancelled
            "#,
            *event_id,
            seq as i64,
            put.title,
            put.extra_description,
            put.starts_at.map(PrimitiveDateTime::from),
            put.ends_at.flatten().map(PrimitiveDateTime::from),
            put.cancelled.unwrap_or(false)
        )
        .fetch_one(conn.ext())
        .await?;

        Ok(overwrite.into())
    }

    async fn calendar_overwrite_get(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
    ) -> Result<CalendarOverwrite> {
        let mut conn = self.acquire().await?;
        let overwrite = query_as!(
            DbCalendarOverwrite,
            r#"
            SELECT event_id, seq as "seq!", title, description, location, url, start_at, end_at, cancelled
            FROM calendar_overwrite
            WHERE event_id = $1 AND seq = $2
            "#,
            *event_id,
            seq as i64
        )
        .fetch_one(conn.ext())
        .await?;

        Ok(overwrite.into())
    }

    async fn calendar_overwrite_list(
        &mut self,
        event_id: CalendarEventId,
    ) -> Result<Vec<CalendarOverwrite>> {
        let mut conn = self.acquire().await?;
        let overwrites = query_as!(
            DbCalendarOverwrite,
            r#"
            SELECT event_id, seq as "seq!", title, description, location, url, start_at, end_at, cancelled
            FROM calendar_overwrite
            WHERE event_id = $1
            "#,
            *event_id
        )
        .fetch_all(conn.ext())
        .await?;

        Ok(overwrites.into_iter().map(Into::into).collect())
    }

    async fn calendar_overwrite_delete(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "DELETE FROM calendar_overwrite WHERE event_id = $1 AND seq = $2",
            *event_id,
            seq as i64
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn calendar_overwrite_rsvp_put(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
        attending: bool,
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        // Ensure overwrite row exists to satisfy foreign key
        query!(
            "INSERT INTO calendar_overwrite (event_id, seq) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            *event_id,
            seq as i64
        )
        .execute(tx.ext())
        .await?;

        query!(
            "INSERT INTO calendar_overwrite_rsvp (event_id, seq, user_id, attending) VALUES ($1, $2, $3, $4)
             ON CONFLICT (event_id, seq, user_id) DO UPDATE SET attending = $4",
            *event_id,
            seq as i64,
            *user_id,
            attending
        )
        .execute(tx.ext())
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn calendar_overwrite_rsvp_delete(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let series_rsvped = query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM calendar_event_rsvp WHERE event_id = $1 AND user_id = $2)",
            *event_id,
            *user_id
        )
        .fetch_one(conn.ext())
        .await?
        .unwrap_or(false);

        if series_rsvped {
            // Upsert an explicit "not attending" record for this overwrite
            self.calendar_overwrite_rsvp_put(event_id, seq, user_id, false)
                .await?;
        } else {
            // Just delete the overwrite rsvp, reverting to series default (not attending)
            query!(
                "DELETE FROM calendar_overwrite_rsvp WHERE event_id = $1 AND seq = $2 AND user_id = $3",
                *event_id,
                seq as i64,
                *user_id
            )
            .execute(conn.ext())
            .await?;
        }

        Ok(())
    }

    async fn calendar_overwrite_rsvp_list(
        &mut self,
        event_id: CalendarEventId,
        seq: u64,
        _query: CalendarEventParticipantQuery,
    ) -> Result<Vec<CalendarEventParticipant>> {
        let mut conn = self.acquire().await?;
        let parent_rsvps: Vec<Uuid> = query_scalar!(
            "SELECT user_id FROM calendar_event_rsvp WHERE event_id = $1",
            *event_id
        )
        .fetch_all(conn.ext())
        .await?;

        struct OverwriteRsvp {
            user_id: Uuid,
            attending: bool,
        }

        let overwrite_rsvps = query_as!(
            OverwriteRsvp,
            "SELECT user_id, attending FROM calendar_overwrite_rsvp WHERE event_id = $1 AND seq = $2",
            *event_id,
            seq as i64
        )
        .fetch_all(conn.ext())
        .await?;

        let mut participants: HashSet<Uuid> = parent_rsvps.into_iter().collect();

        for r in overwrite_rsvps {
            if r.attending {
                participants.insert(r.user_id);
            } else {
                participants.remove(&r.user_id);
            }
        }

        Ok(participants
            .into_iter()
            .map(|uid| CalendarEventParticipant {
                user_id: uid.into(),
                status: CalendarRsvpStatus::Interested,
                user: None,
                member: None,
            })
            .collect())
    }
}
