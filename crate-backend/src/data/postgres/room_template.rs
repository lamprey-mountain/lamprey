use async_trait::async_trait;
use nanoid::nanoid;
use sqlx::{query, query_as, Acquire};
use uuid::Uuid;

use crate::{
    data::{postgres::Pagination, DataRoomTemplate},
    error::{Error, Result},
    gen_paginate,
    types::{DbRoomTemplate, PaginationDirection, PaginationResponse},
};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::room_template::{RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch};
use common::v1::types::UserId;

use super::Postgres;

#[async_trait]
impl DataRoomTemplate for Postgres {
    async fn room_template_create(
        &self,
        creator_id: UserId,
        snapshot: serde_json::Value,
        create: RoomTemplateCreate,
    ) -> Result<DbRoomTemplate> {
        let code = RoomTemplateCode(nanoid!(12));
        let mut tx = self.pool.begin().await?;

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
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.room_template_get(code).await
    }

    async fn room_template_get(&self, code: RoomTemplateCode) -> Result<DbRoomTemplate> {
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
                rt.snapshot
            FROM room_templates rt
            WHERE rt.code = $1
            "#,
            code.0,
        )
        .fetch_one(&self.pool)
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
        &self,
        creator_id: UserId,
        pagination: common::v1::types::PaginationQuery<RoomTemplateCode>,
    ) -> Result<common::v1::types::PaginationResponse<DbRoomTemplate>> {
        let p: Pagination<_> = pagination.try_into()?;

        gen_paginate!(
            p,
            self.pool,
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
                    rt.snapshot
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
            |i: &DbRoomTemplate| i.code.clone()
        )
    }

    async fn room_template_update(
        &self,
        code: RoomTemplateCode,
        patch: RoomTemplatePatch,
    ) -> Result<DbRoomTemplate> {
        let mut tx = self.pool.begin().await?;

        let existing = query!(
            r#"
            SELECT name, description
            FROM room_templates
            WHERE code = $1
            FOR UPDATE
            "#,
            code.0,
        )
        .fetch_one(&mut *tx)
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
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.room_template_get(code).await
    }

    async fn room_template_update_snapshot(
        &self,
        code: RoomTemplateCode,
        snapshot: serde_json::Value,
    ) -> Result<DbRoomTemplate> {
        let mut tx = self.pool.begin().await?;

        query!(
            r#"
            UPDATE room_templates
            SET snapshot = $1, updated_at = NOW()
            WHERE code = $2 AND source_room_id IS NOT NULL
            "#,
            snapshot,
            code.0,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.room_template_get(code).await
    }

    async fn room_template_delete(&self, code: RoomTemplateCode) -> Result<()> {
        query!(r#"DELETE FROM room_templates WHERE code = $1"#, code.0,)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
