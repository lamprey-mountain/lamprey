use async_trait::async_trait;
use nanoid::nanoid;
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::{
    data::{postgres::Pagination, DataRoomTemplate},
    error::{Error, Result},
    gen_paginate,
    types::{DbRoomTemplate, PaginationDirection, PaginationResponse},
};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::room_template::{RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch};
use common::v1::types::{RoomId, UserId};

use super::Postgres;

#[async_trait]
impl DataRoomTemplate for Postgres {
    async fn room_template_create(
        &mut self,
        creator_id: UserId,
        snapshot: serde_json::Value,
        create: RoomTemplateCreate,
    ) -> Result<DbRoomTemplate> {
        let code = RoomTemplateCode(nanoid!(12));
        let mut tx = self.begin_tx().await?;

        query!(
            r#"
            INSERT INTO room_templates (code, name, description, creator_id, source_room_id, snapshot)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            code.0,
            create.name,
            create.description,
            *creator_id,
            Some::<Uuid>(create.room_id.into()),
            snapshot,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;

        self.room_template_get(code).await
    }

    async fn room_template_get(&mut self, code: RoomTemplateCode) -> Result<DbRoomTemplate> {
        let mut conn = self.acquire().await?;
        let row = query_as!(
            DbRoomTemplate,
            r#"
            SELECT
                rt.code,
                rt.name,
                rt.description,
                rt.created_at,
                rt.updated_at,
                rt.creator_id,
                rt.source_room_id,
                rt.snapshot,
                rt.dirty
            FROM room_templates rt
            WHERE rt.code = $1
            "#,
            code.0,
        )
        .fetch_one(conn.ext())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoomTemplate))
            }
            e => Error::Sqlx(e),
        })?;

        Ok(row)
    }

    async fn room_template_list(
        &mut self,
        creator_id: UserId,
        pagination: common::v1::types::PaginationQuery<RoomTemplateCode>,
    ) -> Result<common::v1::types::PaginationResponse<DbRoomTemplate>> {
        let p: Pagination<_> = pagination.try_into()?;

        gen_paginate!(
            p,
            self,
            sqlx::query_as!(
                DbRoomTemplate,
                r#"
                SELECT
                    rt.code,
                    rt.name,
                    rt.description,
                    rt.created_at,
                    rt.updated_at,
                    rt.creator_id,
                    rt.source_room_id,
                    rt.snapshot,
                    rt.dirty
                FROM room_templates rt
                WHERE rt.creator_id = $1 AND rt.code > $2 AND rt.code < $3
                ORDER BY (CASE WHEN $4 = 'f' THEN rt.code END) ASC, rt.code DESC
                LIMIT $5
                "#,
                *creator_id,
                p.after.0,
                p.before.0,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            sqlx::query_scalar!(
                "SELECT count(*) FROM room_templates WHERE creator_id = $1",
                *creator_id
            ),
            |row: DbRoomTemplate| { row },
            |i: &DbRoomTemplate| RoomTemplateCode(i.code.clone()).0
        )
    }

    async fn room_template_update(
        &mut self,
        code: RoomTemplateCode,
        patch: RoomTemplatePatch,
    ) -> Result<DbRoomTemplate> {
        let mut tx = self.begin_tx().await?;

        let existing = query!(
            r#"
            SELECT name, description
            FROM room_templates
            WHERE code = $1
            FOR UPDATE
            "#,
            code.0,
        )
        .fetch_one(tx.ext())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoomTemplate))
            }
            e => Error::Sqlx(e),
        })?;

        let new_name = patch.name.unwrap_or(existing.name);
        let new_description = patch.description.unwrap_or(existing.description);

        query!(
            r#"
            UPDATE room_templates
            SET name = $1, description = $2, updated_at = NOW()
            WHERE code = $3
            "#,
            new_name,
            new_description,
            code.0,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;

        self.room_template_get(code).await
    }

    async fn room_template_update_snapshot(
        &mut self,
        code: RoomTemplateCode,
        snapshot: serde_json::Value,
    ) -> Result<DbRoomTemplate> {
        let mut tx = self.begin_tx().await?;

        query!(
            r#"
            UPDATE room_templates
            SET snapshot = $1, updated_at = NOW(), dirty = false
            WHERE code = $2 AND source_room_id IS NOT NULL
            "#,
            snapshot,
            code.0,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;

        self.room_template_get(code).await
    }

    async fn room_template_mark_dirty(&mut self, source_room_id: RoomId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            UPDATE room_templates
            SET dirty = true, updated_at = NOW()
            WHERE source_room_id = $1
            "#,
            *source_room_id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn room_template_delete(&mut self, code: RoomTemplateCode) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(r#"DELETE FROM room_templates WHERE code = $1"#, code.0,)
            .execute(conn.ext())
            .await?;
        Ok(())
    }
}
