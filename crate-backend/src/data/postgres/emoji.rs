use async_trait::async_trait;
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiOwner};
use common::v1::types::EmojiId;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::data::DataEmoji;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::{PaginationDirection, PaginationQuery, PaginationResponse, RoomId, UserId};

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
            creator_id: value.creator_id.into(),
            owner: match value.room_id {
                Some(id) => EmojiOwner::Room { room_id: id.into() },
                None => EmojiOwner::User,
            },
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
        let mut conn = self.pool.acquire().await?;
        let emoji_id = Uuid::now_v7();
        query!(
            "
    	    INSERT INTO custom_emoji (id, creator_id ,name, media_id, room_id, animated, owner)
    	    VALUES ($1, $2, $3, $4, $5, false, 'Room')
        ",
            emoji_id,
            *creator_id,
            create.name,
            *create.media_id,
            *room_id,
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted room");
        self.emoji_get(emoji_id.into()).await
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
        .await?;
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

    async fn emoji_update(&self, _emoji_id: EmojiId, _patch: EmojiCustomPatch) -> Result<()> {
        // TODO: version id on emoji?
        todo!()
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
}
