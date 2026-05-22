use async_trait::async_trait;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{SessionImprint, SessionPatch, SessionStatus, SessionToken};
use lamprey_backend_core::Error;
use sqlx::{query, query_as, query_scalar};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbSession, DbSessionCreate, DbSessionStatus, PaginationDirection, PaginationQuery,
    PaginationResponse, Session, SessionId, UserId,
};

use crate::data::DataSession;

use super::{Pagination, Postgres};

#[async_trait]
impl DataSession for Postgres {
    async fn session_create(&mut self, create: DbSessionCreate) -> Result<Session> {
        let mut conn = self.acquire().await?;
        let session_id = Uuid::now_v7();
        let session = query_as!(
            DbSession,
            r#"
            INSERT INTO session (id, user_id, token, status, name, expires_at, type, application_id, last_seen_at, ip_addr, user_agent, country_code, country_name, city_name)
            VALUES ($1, NULL, $2, 'Unauthorized', $3, $4, $5, $6, now(), $7::text::inet, $8, NULL, NULL, NULL)
            RETURNING id, user_id, token, status as "status: _", name, expires_at, type as ty, application_id, last_seen_at, ip_addr::text, user_agent, country_code, country_name, city_name, authorized_at, deauthorized_at"#,
            session_id,
            create.token.0,
            create.name,
            create.expires_at.map(PrimitiveDateTime::from),
            create.ty.to_string(),
            create.application_id.map(|id| id.into_inner()),
            create.ip_addr,
            create.user_agent,
        )
        .fetch_one(conn.ext())
        .await?;
        Ok(session.into())
    }

    async fn session_get(&mut self, id: SessionId) -> Result<Session> {
        let mut conn = self.acquire().await?;
        tracing::debug!("session_get: {:?}", id);
        let session = query_as!(
            DbSession,
            r#"SELECT id, user_id, token, status as "status: _", name, expires_at, type as ty, application_id, last_seen_at, ip_addr::text, user_agent, country_code, country_name, city_name, authorized_at, deauthorized_at FROM session WHERE id = $1"#,
            *id,
        )
        .fetch_one(conn.ext())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                tracing::debug!("session_get: row not found for {:?}", id);
                Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownSession,
                ))
            }
            e => Error::Sqlx(e),
        })?;
        Ok(session.into())
    }

    async fn session_get_by_token(&mut self, token: SessionToken) -> Result<Session> {
        let mut conn = self.acquire().await?;
        tracing::debug!("session_get_by_token: {:?}", token);
        let session = query_as!(
            DbSession,
            r#"SELECT id, user_id, token, status as "status: _", name, expires_at, type as ty, application_id, last_seen_at, ip_addr::text, user_agent, country_code, country_name, city_name, authorized_at, deauthorized_at FROM session WHERE token = $1"#,
            token.0
        )
            .fetch_one(conn.ext())
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    tracing::debug!("session_get_by_token: row not found for {:?}", token);
                    Error::ApiError(ApiError::from_code(
                        ErrorCode::UnknownSession,
                    ))
                }
                e => Error::Sqlx(e),
            })?;
        Ok(session.into())
    }

    async fn session_set_status(
        &mut self,
        session_id: SessionId,
        status: SessionStatus,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        let user_id = status.user_id().map(|i| *i);
        let is_authorized = matches!(
            status,
            SessionStatus::Authorized { .. } | SessionStatus::Sudo { .. }
        );
        let status_db: DbSessionStatus = status.into();
        query!(
            r#"UPDATE session SET
            status = $2,
            user_id = $3,
            authorized_at = (CASE WHEN $4 THEN COALESCE(authorized_at, now()) ELSE authorized_at END),
            deauthorized_at = (CASE WHEN $4 THEN NULL ELSE COALESCE(deauthorized_at, now()) END)
            WHERE id = $1"#,
            *session_id,
            status_db as _,
            user_id,
            is_authorized,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn session_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_as!(
                DbSession,
                r#"
        	SELECT id, user_id, token, status as "status: _", name, expires_at, type as ty, application_id, last_seen_at, ip_addr::text, user_agent, country_code, country_name, city_name, authorized_at, deauthorized_at FROM session
        	WHERE user_id = $1 AND id > $2 AND id < $3 AND status != 'Unauthorized'
        	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
        	"#,
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM session WHERE user_id = $1 AND status != 'Unauthorized'",
                *user_id
            ),
            |i: &Session| i.id.to_string()
        )
    }

    async fn session_delete(&mut self, session_id: SessionId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"DELETE FROM session WHERE id = $1"#,
            session_id.into_inner()
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn session_delete_all(&mut self, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!("DELETE FROM session WHERE user_id = $1", *user_id)
            .execute(conn.ext())
            .await?;
        Ok(())
    }

    async fn session_update(&mut self, session_id: SessionId, patch: SessionPatch) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        let session = query_as!(
            DbSession,
            r#"
            SELECT id, user_id, token, status as "status: _", name, expires_at, type as ty, application_id, last_seen_at, ip_addr::text, user_agent, country_code, country_name, city_name, authorized_at, deauthorized_at
            FROM session
            WHERE id = $1
            FOR UPDATE
            "#,
            session_id.into_inner()
        )
        .fetch_one(tx.ext())
        .await?;
        query!(
            "UPDATE session SET name = $2 WHERE id = $1",
            *session_id,
            patch.name.unwrap_or(session.name),
        )
        .execute(tx.ext())
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn session_set_last_seen_at(&mut self, session_id: SessionId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "UPDATE session SET last_seen_at = now() WHERE id = $1",
            *session_id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn session_update_imprint(
        &mut self,
        session_id: SessionId,
        imprint: SessionImprint,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            UPDATE session 
            SET last_seen_at = $2, ip_addr = $3::text::inet, user_agent = $4, country_code = $5, country_name = $6, city_name = $7
            WHERE id = $1
            "#,
            *session_id,
            PrimitiveDateTime::from(imprint.last_seen_at),
            imprint.ip_addr,
            imprint.user_agent,
            imprint.country_code,
            imprint.country_name,
            imprint.city_name,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }
}
