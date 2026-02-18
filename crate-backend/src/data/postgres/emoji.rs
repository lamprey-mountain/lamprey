use async_trait::async_trait;
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiOwner};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::EmojiId;
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::consts::MAX_CUSTOM_EMOJI;
use crate::data::DataEmoji;
use crate::error::Result;
use crate::types::{
    MediaLinkType, PaginationDirection, PaginationQuery, PaginationResponse, RoomId, UserId,
};
use crate::{gen_paginate, Error};

use super::{Pagination, Postgres};

struct DbEmojiCustom {
    id: Uuid,
    name: String,
    creator_id: Uuid,
    animated: bool,
    media_id: Uuid,
    room_id: Option<Uuid>,
}

impl From<DbEmojiCustom> for EmojiCustom {
    fn from(value: DbEmojiCustom) -> Self {
        EmojiCustom {
            id: value.id.into(),
            name: value.name,
            creator_id: Some(value.creator_id.into()),
            owner: Some(match value.room_id {
                Some(id) => EmojiOwner::Room { room_id: id.into() },
                None => EmojiOwner::User,
            }),
            animated: value.animated,
            media_id: value.media_id.into(),
        }
    }
}

#[async_trait]
impl DataEmoji for Postgres {
    async fn emoji_create(
        &self,
        creator_id: UserId,
        room_id: RoomId,
        create: EmojiCustomCreate,
    ) -> Result<EmojiCustom> {
        let mut tx = self.pool.begin().await?;

        let links = sqlx::query!(
            "SELECT media_id FROM media_link WHERE media_id = $1",
            *create.media_id
        )
        .fetch_all(&mut *tx)
        .await?;

        if !links.is_empty() {
            return Err(Error::BadStatic("media already used"));
        }

        let count = query_scalar!(
            "SELECT count(*) FROM custom_emoji WHERE room_id = $1",
            *room_id,
        )
        .fetch_optional(&mut *tx)
        .await?
        .flatten()
        .unwrap_or(0) as u32;
        if count >= MAX_CUSTOM_EMOJI {
            return Err(Error::BadStatic(
                "max number of custom emoji reached (1024)",
            ));
        }

        let emoji_id = EmojiId::new();

        query!(
            "
    	    INSERT INTO custom_emoji (id, creator_id ,name, media_id, room_id, animated, owner)
    	    VALUES ($1, $2, $3, $4, $5, false, 'Room')
        ",
            *emoji_id,
            *creator_id,
            create.name,
            *create.media_id,
            *room_id,
        )
        .execute(&mut *tx)
        .await?;

        query!(
            r#"
    	    INSERT INTO media_link (media_id, target_id, link_type)
    	    VALUES ($1, $2, $3)
        "#,
            *create.media_id,
            *emoji_id,
            MediaLinkType::CustomEmoji as _
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.emoji_get(emoji_id).await
    }

    async fn emoji_get(&self, emoji_id: EmojiId) -> Result<EmojiCustom> {
        let id: Uuid = emoji_id.into();
        let mut conn = self.pool.acquire().await?;
        let row = query_as!(
            DbEmojiCustom,
            r#"SELECT id, name, creator_id, animated, media_id, room_id FROM custom_emoji WHERE id = $1"#,
            id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownEmoji,
            )),
            e => Error::Sqlx(e),
        })?;
        Ok(row.into())
    }

    async fn emoji_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbEmojiCustom,
                r#"
                SELECT id, name, creator_id, animated, media_id, room_id
                FROM custom_emoji
            	WHERE room_id = $1 AND id > $2 AND id < $3 AND deleted_at IS NULL
            	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
                "#,
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM custom_emoji WHERE room_id = $1",
                *room_id
            ),
            |i: &EmojiCustom| i.id.to_string()
        )
    }

    async fn emoji_update(&self, emoji_id: EmojiId, patch: EmojiCustomPatch) -> Result<()> {
        if let Some(name) = patch.name {
            query!(
                "UPDATE custom_emoji SET name = $1 WHERE id = $2",
                name,
                *emoji_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn emoji_delete(&self, emoji_id: EmojiId) -> Result<()> {
        query!(
            "UPDATE custom_emoji SET deleted_at = now() WHERE id = $1",
            *emoji_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn emoji_search(
        &self,
        user_id: UserId,
        query: String,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>> {
        let p: Pagination<_> = pagination.try_into()?;
        let query = format!("%{}%", query);

        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbEmojiCustom,
                r#"
                SELECT e.id, e.name, e.creator_id, e.animated, e.media_id, e.room_id
                FROM custom_emoji e
                JOIN room_member rm ON e.room_id = rm.room_id
                WHERE rm.user_id = $1 AND rm.membership = 'Join' AND e.name ILIKE $2
                AND e.id > $3 AND e.id < $4 AND e.deleted_at IS NULL
            	ORDER BY (CASE WHEN $5 = 'f' THEN e.id END), e.id DESC LIMIT $6
                "#,
                *user_id,
                query,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"
                SELECT count(*)
                FROM custom_emoji e
                JOIN room_member rm ON e.room_id = rm.room_id
                WHERE rm.user_id = $1 AND rm.membership = 'Join' AND e.name ILIKE $2
                AND e.deleted_at IS NULL
                "#,
                *user_id,
                query
            ),
            |i: &EmojiCustom| i.id.to_string()
        )
    }
}
