use async_trait::async_trait;
use common::v1::types::{SessionPatch, SessionStatus, SessionToken};
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbSession, DbSessionStatus, PaginationDirection, PaginationQuery, PaginationResponse, Session,
    SessionId, UserId,
};

use crate::data::DataSession;

use super::{Pagination, Postgres};

#[async_trait]
impl DataSession for Postgres {
    async fn session_create(&self, token: SessionToken, name: Option<String>) -> Result<Session> {
        let session_id = Uuid::now_v7();
        let session = query_as!(
            DbSession,
            r#"
            INSERT INTO session (id, user_id, token, status, name)
            VALUES ($1, NULL, $2, 'Unauthorized', $3)
            RETURNING id, user_id, token, status as "status: _", name"#,
            session_id,
            token.0,
            name,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(session.into())
    }

    async fn session_get(&self, id: SessionId) -> Result<Session> {
        let session = query_as!(
            DbSession,
            r#"SELECT id, user_id, token, status as "status: _", name FROM session WHERE id = $1"#,
            id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(session.into())
    }

    async fn session_get_by_token(&self, token: SessionToken) -> Result<Session> {
        let session = query_as!(
            DbSession,
            r#"SELECT id, user_id, token, status as "status: _", name FROM session WHERE token = $1"#,
            token.0
        )
            .fetch_one(&self.pool)
            .await?;
        Ok(session.into())
    }

    async fn session_set_status(&self, session_id: SessionId, status: SessionStatus) -> Result<()> {
        let user_id = status.user_id().map(|i| i.into_inner());
        let status_db: DbSessionStatus = status.into();
        query!(
            r#"UPDATE session SET status = $2, user_id = $3 WHERE id = $1"#,
            session_id.into_inner(),
            status_db as _,
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn session_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbSession,
                r#"
        	SELECT id, user_id, token, status as "status: _", name FROM session
        	WHERE user_id = $1 AND id > $2 AND id < $3 AND status != 'Unauthorized'
        	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
        	"#,
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM session WHERE user_id = $1 AND status != 'Unauthorized'",
                user_id.into_inner()
            )
        )
    }

    async fn session_delete(&self, session_id: SessionId) -> Result<()> {
        query!(
            r#"DELETE FROM session WHERE id = $1"#,
            session_id.into_inner()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn session_update(&self, session_id: SessionId, patch: SessionPatch) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let session = query_as!(
            DbSession,
            r#"
            SELECT id, user_id, token, status as "status: _", name
            FROM session
            WHERE id = $1
            FOR UPDATE
            "#,
            session_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        query!(
            "UPDATE session SET name = $2 WHERE id = $1",
            session_id.into_inner(),
            patch.name.unwrap_or(session.name),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
