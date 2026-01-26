use async_trait::async_trait;
use common::v1::types::calendar::{Calendar, CalendarPatch, Timezone};
use common::v1::types::document::{
    Document, DocumentArchived, DocumentPatch, DocumentPublished, Wiki, WikiPatch,
};
use common::v1::types::misc::Color;
use common::v1::types::util::{Diff, Time};
use common::v1::types::{ChannelReorder, RoomVerId};
use sqlx::{query, query_file_as, query_scalar, Acquire};
use time::PrimitiveDateTime;
use tracing::{info, warn};

use crate::error::Result;
use crate::types::{
    Channel, ChannelId, ChannelPatch, ChannelVerId, DbChannel, DbChannelCalendar, DbChannelCreate,
    DbChannelDocument, DbChannelPrivate, DbChannelType, DbChannelWiki, PaginationDirection,
    PaginationQuery, PaginationResponse, RoomId, UserId,
};
use crate::{gen_paginate, Error};

use crate::data::DataChannel;

use super::{Pagination, Postgres};

#[async_trait]
impl DataChannel for Postgres {
    async fn channel_create(&self, create: DbChannelCreate) -> Result<ChannelId> {
        let channel_id = ChannelId::new();
        self.channel_create_with_id(channel_id, create).await?;
        Ok(channel_id)
    }

    async fn channel_create_with_id(
        &self,
        channel_id: ChannelId,
        create: DbChannelCreate,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) = create.room_id {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM channel WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_CHANNEL_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active channels (max {})",
                    crate::consts::MAX_CHANNEL_COUNT
                )));
            }
        }

        query!(
            "
			INSERT INTO channel (id, version_id, creator_id, room_id, name, description, type, nsfw, locked, bitrate, user_limit, parent_id, owner_id, icon, invitable, auto_archive_duration, default_auto_archive_duration, slowmode_thread, slowmode_message, default_slowmode_message, url, locked_until, locked_roles)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $21, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, null, $22)
        ",
            channel_id.into_inner(),
            channel_id.into_inner(),
            create.creator_id.into_inner(),
            create.room_id,
            create.name,
            create.description,
            create.ty as _,
            create.nsfw,
            create.bitrate,
            create.user_limit,
            create.parent_id,
            create.owner_id,
            create.icon,
            create.invitable,
            create.auto_archive_duration,
            create.default_auto_archive_duration,
            create.slowmode_thread.map(|s| s as i32),
            create.slowmode_message.map(|s| s as i32),
            create.default_slowmode_message.map(|s| s as i32),
            create.url,
            false,
            &[],
        )
        .execute(&mut *tx)
        .await?;

        if let Some(tags) = &create.tags {
            if !tags.is_empty() {
                let tag_ids: Vec<_> = tags.iter().map(|t| t.into_inner()).collect();
                query!(
                    "INSERT INTO channel_tag (channel_id, tag_id) SELECT $1, * FROM UNNEST($2::uuid[])",
                    channel_id.into_inner(),
                    &tag_ids
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        info!("inserted channel");
        Ok(())
    }

    async fn channel_get(&self, channel_id: ChannelId) -> Result<Channel> {
        let thread = query_file_as!(DbChannel, "sql/channel_get.sql", channel_id.into_inner())
            .fetch_one(&self.pool)
            .await?;
        Ok(thread.into())
    }

    async fn channel_get_many(&self, channel_ids: &[ChannelId]) -> Result<Vec<Channel>> {
        let ids: Vec<uuid::Uuid> = channel_ids.iter().map(|id| id.into_inner()).collect();
        let threads = query_file_as!(DbChannel, "sql/channel_get_many.sql", &ids)
            .fetch_all(&self.pool)
            .await?;
        Ok(threads.into_iter().map(Into::into).collect())
    }

    async fn channel_list(&self, room_id: RoomId) -> Result<Vec<Channel>> {
        let channels = query_file_as!(DbChannel, "sql/channel_list.sql", *room_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(channels.into_iter().map(Into::into).collect())
    }

    async fn channel_list_removed(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/channel_paginate_removed.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                parent_id.map(|id| *id),
            ),
            query_scalar!(
                r#"SELECT count(*) FROM channel WHERE room_id = $1 AND deleted_at IS NOT NULL AND ($2::uuid IS NULL OR parent_id = $2)"#,
                room_id.into_inner(),
                parent_id.map(|id| *id)
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn channel_get_private(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<DbChannelPrivate> {
        let thread_private = query_file_as!(
            DbChannelPrivate,
            "sql/channel_get_private.sql",
            *thread_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(thread_private)
    }

    async fn channel_update(
        &self,
        thread_id: ChannelId,
        patch: ChannelPatch,
    ) -> Result<ChannelVerId> {
        let mut tx = self.pool.begin().await?;
        let db_chan = query_file_as!(DbChannel, "sql/channel_get.sql", *thread_id)
            .fetch_one(&mut *tx)
            .await?;
        let mut last_activity_at = db_chan.last_activity_at;
        let thread: Channel = db_chan.into();

        if patch.archived == Some(false) && thread.archived_at.is_some() {
            if let Some(room_id) = thread.room_id {
                let count: i64 = query_scalar!(
                    "SELECT count(*) FROM channel WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                    *room_id
                )
                .fetch_one(&mut *tx)
                .await?
                .unwrap_or(0);

                if count as u32 >= crate::consts::MAX_CHANNEL_COUNT {
                    return Err(Error::BadRequest(format!(
                        "too many active channel (max {})",
                        crate::consts::MAX_CHANNEL_COUNT
                    )));
                }
            }
        }

        if let Some(tags) = &patch.tags {
            query!(
                "DELETE FROM channel_tag WHERE channel_id = $1",
                thread_id.into_inner()
            )
            .execute(&mut *tx)
            .await?;

            if !tags.is_empty() {
                let tag_ids: Vec<_> = tags.iter().map(|t| t.into_inner()).collect();
                query!(
                    "INSERT INTO channel_tag (channel_id, tag_id) SELECT $1, * FROM UNNEST($2::uuid[])",
                    thread_id.into_inner(),
                    &tag_ids
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        let version_id = ChannelVerId::new();

        let archived_at = match patch.archived {
            Some(true) => Some(time::OffsetDateTime::now_utc()),
            Some(false) => None,
            None => thread.archived_at.map(|t| *t),
        };

        let new_parent_id = match patch.parent_id {
            Some(id) => id.map(|i| i.into_inner()),
            None => thread.parent_id.map(|i| i.into_inner()),
        };

        let new_ty: DbChannelType = patch.ty.map(Into::into).unwrap_or_else(|| thread.ty.into());

        if patch.archived == Some(false)
            || patch
                .auto_archive_duration
                .changes(&thread.auto_archive_duration)
        {
            let now = time::OffsetDateTime::now_utc();
            let now = PrimitiveDateTime::new(now.date(), now.time());
            last_activity_at = Some(now);
        }

        let locked_val = patch.locked.unwrap_or(thread.locked);
        let locked_bool = locked_val.is_some();
        let locked_until = locked_val.as_ref().and_then(|l| {
            l.until.map(|t| {
                let inner = t.into_inner();
                PrimitiveDateTime::new(inner.date(), inner.time())
            })
        });
        let locked_roles: Vec<uuid::Uuid> = locked_val
            .as_ref()
            .map(|l| l.allow_roles.iter().map(|r| r.into_inner()).collect())
            .unwrap_or_default();

        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                name = $3,
                description = $4,
                nsfw = $5,
                bitrate = $6,
                user_limit = $7,
                owner_id = $8,
                icon = $9,
                locked = $10,
                archived_at = $11,
                invitable = $12,
                type = $13,
                parent_id = $14,
                auto_archive_duration = $15,
                default_auto_archive_duration = $16,
                slowmode_thread = $17,
                slowmode_message = $18,
                default_slowmode_message = $19,
                last_activity_at = $20,
                url = $21,
                locked_until = $22,
                locked_roles = $23
            WHERE id = $1
        "#,
            thread_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(thread.name),
            patch.description.unwrap_or(thread.description),
            patch.nsfw.unwrap_or(thread.nsfw),
            patch.bitrate.unwrap_or(thread.bitrate).map(|i| i as i32),
            patch
                .user_limit
                .unwrap_or(thread.user_limit)
                .map(|i| i as i32),
            patch
                .owner_id
                .unwrap_or(thread.owner_id)
                .map(|i| i.into_inner()),
            patch.icon.unwrap_or(thread.icon).map(|id| *id),
            locked_bool,
            archived_at as _,
            patch.invitable.unwrap_or(thread.invitable),
            new_ty as _,
            new_parent_id,
            patch
                .auto_archive_duration
                .unwrap_or(thread.auto_archive_duration)
                .map(|i| i as i64),
            patch
                .default_auto_archive_duration
                .unwrap_or(thread.default_auto_archive_duration)
                .map(|i| i as i64),
            patch
                .slowmode_thread
                .unwrap_or(thread.slowmode_thread)
                .map(|i| i as i32),
            patch
                .slowmode_message
                .unwrap_or(thread.slowmode_message)
                .map(|i| i as i32),
            patch
                .default_slowmode_message
                .unwrap_or(thread.default_slowmode_message)
                .map(|i| i as i32),
            last_activity_at as _,
            patch.url.unwrap_or(thread.url),
            locked_until,
            &locked_roles,
        )
        .execute(&mut *tx)
        .await?;

        if let Some(ref document_patch) = patch.document {
            let doc_exists = query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM channel_document WHERE channel_id = $1)",
                thread_id.into_inner()
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);

            if !doc_exists {
                query!(
                    "INSERT INTO channel_document (channel_id) VALUES ($1)",
                    *thread_id
                )
                .execute(&mut *tx)
                .await?;
            }

            self.channel_document_update_impl(&mut tx, thread_id, document_patch)
                .await?;
        }

        if let Some(ref wiki_patch) = patch.wiki {
            let wiki_exists = query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM channel_wiki WHERE channel_id = $1)",
                thread_id.into_inner()
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);

            if !wiki_exists {
                query!(
                    "INSERT INTO channel_wiki (channel_id) VALUES ($1)",
                    *thread_id
                )
                .execute(&mut *tx)
                .await?;
            }

            self.channel_wiki_update_impl(&mut tx, thread_id, wiki_patch)
                .await?;
        }

        if let Some(ref calendar_patch) = patch.calendar {
            let calendar_exists = query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM channel_calendar WHERE channel_id = $1)",
                *thread_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);

            if !calendar_exists {
                query!(
                    "INSERT INTO channel_calendar (channel_id) VALUES ($1)",
                    thread_id.into_inner()
                )
                .execute(&mut *tx)
                .await?;
            }

            self.channel_calendar_update_impl(&mut tx, thread_id, calendar_patch)
                .await?;
        }

        tx.commit().await?;
        Ok(version_id)
    }

    async fn channel_delete(&self, thread_id: ChannelId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let version_id = ChannelVerId::new();
        let room_version_id = RoomVerId::new();

        query!(
            r#"
            UPDATE room SET
                version_id = $2,
                welcome_channel_id = NULL
            WHERE welcome_channel_id = $1
            "#,
            *thread_id,
            *room_version_id,
        )
        .execute(&mut *tx)
        .await?;

        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                deleted_at = NOW()
            WHERE id = $1
            "#,
            thread_id.into_inner(),
            version_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_undelete(&self, thread_id: ChannelId) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) =
            query_scalar!("SELECT room_id FROM channel WHERE id = $1", *thread_id)
                .fetch_one(&mut *tx)
                .await?
        {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM channel WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_CHANNEL_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active channel (max {})",
                    crate::consts::MAX_CHANNEL_COUNT
                )));
            }
        }

        let version_id = ChannelVerId::new();
        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                deleted_at = NULL
            WHERE id = $1
            "#,
            thread_id.into_inner(),
            version_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_reorder(&self, data: ChannelReorder) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        for thread in data.channels {
            let old = query!(
                r#"SELECT position, parent_id FROM channel WHERE id = $1"#,
                *thread.id,
            )
            .fetch_one(&mut *tx)
            .await?;
            let new_position = thread
                .position
                .map(|i| i.map(|i| i as i32))
                .unwrap_or(old.position);

            let new_parent_id = thread
                .parent_id
                .map(|i| i.map(|i| *i))
                .unwrap_or(old.parent_id);

            if new_position != old.position || new_parent_id != old.parent_id {
                let version_id = ChannelVerId::new();
                query!(
                    r#"UPDATE channel SET version_id = $2, position = $3, parent_id = $4 WHERE id = $1"#,
                    *thread.id,
                    *version_id,
                    thread.position.map(|i| i.map(|i| i as i32)).unwrap_or(old.position),
                    thread.parent_id.map(|i| i.map(|i| *i)).unwrap_or(old.parent_id),
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn channel_upgrade_gdm(&self, thread_id: ChannelId, room_id: RoomId) -> Result<()> {
        let version_id = ChannelVerId::new();
        let ty = DbChannelType::Text;
        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                room_id = $3,
                type = $4
            WHERE id = $1
            "#,
            *thread_id,
            *version_id,
            *room_id,
            ty as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn channel_get_message_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>> {
        let row = query_scalar!(
            "SELECT expires_at FROM channel_slowmode_message WHERE channel_id = $1 AND user_id = $2",
            *channel_id,
            *user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Time::from))
    }

    async fn channel_set_message_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()> {
        query!(
            "INSERT INTO channel_slowmode_message (channel_id, user_id, expires_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (channel_id, user_id)
             DO UPDATE SET expires_at = $3",
            *channel_id,
            *user_id,
            time::PrimitiveDateTime::new(
                expires_at.into_inner().date(),
                expires_at.into_inner().time()
            )
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn channel_get_thread_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>> {
        let row = query_scalar!(
            "SELECT expires_at FROM channel_slowmode_thread WHERE channel_id = $1 AND user_id = $2",
            *channel_id,
            *user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Time::from))
    }

    async fn channel_set_thread_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()> {
        query!(
            "INSERT INTO channel_slowmode_thread (channel_id, user_id, expires_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (channel_id, user_id)
             DO UPDATE SET expires_at = $3",
            *channel_id,
            *user_id,
            time::PrimitiveDateTime::new(
                expires_at.into_inner().date(),
                expires_at.into_inner().time()
            )
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn channel_document_insert(
        &self,
        channel_id: ChannelId,
        document: &Document,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.channel_document_insert_impl(&mut tx, channel_id, document)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_document_get(&self, channel_id: ChannelId) -> Result<Option<Document>> {
        let row = query!(
            r#"
            SELECT draft, archived_at, archived_reason, template, slug,
                   published_at, published_revision, published_unlisted
            FROM channel_document
            WHERE channel_id = $1
            "#,
            channel_id.into_inner()
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(doc) = row {
            let archived = if doc.archived_at.is_some() {
                Some(DocumentArchived {
                    archived_at: Time::from(PrimitiveDateTime::new(
                        doc.archived_at.unwrap().date(),
                        doc.archived_at.unwrap().time(),
                    )),
                    reason: doc.archived_reason,
                })
            } else {
                None
            };

            let published = if doc.published_at.is_some() {
                Some(DocumentPublished {
                    time: Time::from(PrimitiveDateTime::new(
                        doc.published_at.unwrap().date(),
                        doc.published_at.unwrap().time(),
                    )),
                    revision: doc.published_revision.unwrap().parse().map_err(|_| {
                        Error::Internal("Invalid document revision format".to_string())
                    })?,
                    unlisted: doc.published_unlisted.unwrap_or(false),
                })
            } else {
                None
            };

            Ok(Some(Document {
                draft: doc.draft,
                archived,
                template: doc.template,
                slug: doc.slug,
                published,
            }))
        } else {
            Ok(None)
        }
    }

    async fn channel_document_update(
        &self,
        channel_id: ChannelId,
        document_patch: &DocumentPatch,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.channel_document_update_impl(&mut tx, channel_id, document_patch)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_wiki_insert(&self, channel_id: ChannelId, wiki: &Wiki) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.channel_wiki_insert_impl(&mut tx, channel_id, wiki)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_wiki_get(&self, channel_id: ChannelId) -> Result<Option<Wiki>> {
        let row = query!(
            r#"
            SELECT allow_indexing, page_index, page_notfound
            FROM channel_wiki
            WHERE channel_id = $1
            "#,
            channel_id.into_inner()
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(wiki) = row {
            Ok(Some(Wiki {
                allow_indexing: wiki.allow_indexing,
                page_index: wiki.page_index.map(ChannelId::from),
                page_notfound: wiki.page_notfound.map(ChannelId::from),
            }))
        } else {
            Ok(None)
        }
    }

    async fn channel_wiki_update(
        &self,
        channel_id: ChannelId,
        wiki_patch: &WikiPatch,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.channel_wiki_update_impl(&mut tx, channel_id, wiki_patch)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_calendar_insert(
        &self,
        channel_id: ChannelId,
        calendar: &Calendar,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.channel_calendar_insert_impl(&mut tx, channel_id, calendar)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_calendar_get(&self, channel_id: ChannelId) -> Result<Option<Calendar>> {
        let row = query!(
            r#"
            SELECT color, default_timezone
            FROM channel_calendar
            WHERE channel_id = $1
            "#,
            *channel_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(cal) = row {
            Ok(Some(Calendar {
                color: cal.color.map(|c| Color::from_hex_string(c)),
                default_timezone: Timezone(cal.default_timezone),
            }))
        } else {
            Ok(None)
        }
    }

    async fn channel_calendar_update(
        &self,
        channel_id: ChannelId,
        calendar_patch: &CalendarPatch,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        self.channel_calendar_update_impl(&mut tx, channel_id, calendar_patch)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

impl Postgres {
    async fn channel_document_insert_impl(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        channel_id: ChannelId,
        document: &Document,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO channel_document (
                channel_id, draft, archived_at, archived_reason, template, slug,
                published_at, published_revision, published_unlisted
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            *channel_id,
            document.draft,
            document
                .archived
                .as_ref()
                .map(|a| time::PrimitiveDateTime::new(
                    a.archived_at.into_inner().date(),
                    a.archived_at.into_inner().time()
                )),
            document.archived.as_ref().and_then(|a| a.reason.as_deref()),
            document.template,
            document.slug.as_deref(),
            document
                .published
                .as_ref()
                .map(|p| time::PrimitiveDateTime::new(
                    p.time.into_inner().date(),
                    p.time.into_inner().time()
                )),
            document.published.as_ref().map(|p| p.revision.to_string()),
            document.published.as_ref().map(|p| p.unlisted)
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn channel_document_update_impl(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        channel_id: ChannelId,
        document_patch: &DocumentPatch,
    ) -> Result<()> {
        let current_doc = query!(
            r#"
            SELECT draft, archived_at, archived_reason, template, slug,
                   published_at, published_revision, published_unlisted
            FROM channel_document
            WHERE channel_id = $1
            "#,
            channel_id.into_inner()
        )
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(current) = current_doc {
            let archived_at = match document_patch.archived {
                Some(Some(_)) => {
                    let t = Time::now_utc();
                    Some(PrimitiveDateTime::new(
                        t.into_inner().date(),
                        t.into_inner().time(),
                    ))
                }
                Some(None) => None,
                None => current.archived_at,
            };

            let published_at = match document_patch.published {
                Some(Some(_)) => {
                    let t = Time::now_utc();
                    Some(PrimitiveDateTime::new(
                        t.into_inner().date(),
                        t.into_inner().time(),
                    ))
                }
                Some(None) => None,
                None => current.published_at,
            };

            let archived_reason: Option<String> = match &document_patch.archived {
                Some(Some(patch)) => patch.reason.clone().unwrap_or(None),
                Some(None) => None,
                None => current.archived_reason.clone(),
            };

            let published_revision: Option<String> = match &document_patch.published {
                Some(Some(patch)) => patch.revision.as_ref().map(|r| r.to_string()),
                Some(None) => None,
                None => current.published_revision.clone(),
            };

            let published_unlisted: Option<bool> = match &document_patch.published {
                Some(Some(patch)) => patch.unlisted,
                Some(None) => None,
                None => current.published_unlisted,
            };

            query!(
                r#"
                UPDATE channel_document
                SET
                    draft = $2,
                    archived_at = $3,
                    archived_reason = $4,
                    template = $5,
                    slug = $6,
                    published_at = $7,
                    published_revision = $8,
                    published_unlisted = $9
                WHERE channel_id = $1
                "#,
                *channel_id,
                document_patch.draft.unwrap_or(current.draft),
                archived_at as Option<PrimitiveDateTime>,
                archived_reason,
                document_patch.template.unwrap_or(current.template),
                document_patch.slug.clone().unwrap_or(current.slug),
                published_at as Option<PrimitiveDateTime>,
                published_revision,
                published_unlisted,
            )
            .execute(&mut **tx)
            .await?;
        } else {
            warn!("channel_document not found");
        }

        Ok(())
    }

    async fn channel_wiki_insert_impl(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        channel_id: ChannelId,
        wiki: &Wiki,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO channel_wiki (
                channel_id, allow_indexing, page_index, page_notfound
            )
            VALUES ($1, $2, $3, $4)
            "#,
            *channel_id,
            wiki.allow_indexing,
            wiki.page_index.map(|id| *id),
            wiki.page_notfound.map(|id| *id)
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn channel_wiki_update_impl(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        channel_id: ChannelId,
        wiki_patch: &WikiPatch,
    ) -> Result<()> {
        let current_wiki = query!(
            r#"
            SELECT allow_indexing, page_index, page_notfound
            FROM channel_wiki
            WHERE channel_id = $1
            "#,
            channel_id.into_inner()
        )
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(current) = current_wiki {
            query!(
                r#"
                UPDATE channel_wiki
                SET
                    allow_indexing = $2,
                    page_index = $3,
                    page_notfound = $4
                WHERE channel_id = $1
                "#,
                channel_id.into_inner(),
                wiki_patch.allow_indexing.unwrap_or(current.allow_indexing),
                wiki_patch
                    .page_index
                    .map(|p| p.map(|p| *p))
                    .unwrap_or(current.page_index),
                wiki_patch
                    .page_notfound
                    .map(|p| p.map(|p| *p))
                    .unwrap_or(current.page_notfound),
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn channel_calendar_insert_impl(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        channel_id: ChannelId,
        calendar: &Calendar,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO channel_calendar (
                channel_id, color, default_timezone
            )
            VALUES ($1, $2, $3)
            "#,
            channel_id.into_inner(),
            calendar.color.as_ref().map(|c| c.as_ref()),
            calendar.default_timezone.0
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn channel_calendar_update_impl(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        channel_id: ChannelId,
        calendar_patch: &CalendarPatch,
    ) -> Result<()> {
        let current_cal = query!(
            r#"
            SELECT color, default_timezone
            FROM channel_calendar
            WHERE channel_id = $1
            "#,
            *channel_id,
        )
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(current) = current_cal {
            let new_color: Option<&str> = match &calendar_patch.color {
                Some(Some(c)) => Some(c.as_ref()),
                Some(None) => None,
                None => current.color.as_deref(),
            };

            let new_default_timezone = calendar_patch
                .default_timezone
                .as_ref()
                .map(|tz| tz.0.clone())
                .unwrap_or(current.default_timezone.clone());

            query!(
                r#"
                UPDATE channel_calendar
                SET
                    color = $2,
                    default_timezone = $3
                WHERE channel_id = $1
                "#,
                *channel_id,
                new_color,
                new_default_timezone
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }
}
