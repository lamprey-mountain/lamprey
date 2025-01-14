use async_trait::async_trait;
use sqlx::{query_as, Acquire};

use crate::error::{Error, Result};
use crate::types::{PaginationQuery, PaginationResponse, Session, SessionId, ThreadCreate, UserId};

use crate::data::DataSession;

use super::Postgres;

#[async_trait]
impl DataSession for Postgres {
    async fn session_create(&self, create: ThreadCreate) -> Result<SessionId> {
        todo!()
    }

    async fn session_get(&self, id: SessionId) -> Result<Session> {
        let mut conn = self.pool.acquire().await?;
        let session = query_as!(
            Session,
            r#"SELECT id, user_id, token, status as "status: _", name FROM session WHERE id = $1"#,
            id.into_inner()
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| match err.into() {
            Error::NotFound => Error::MissingAuth,
            other => other,
        })?;
        Ok(session)
    }

    async fn session_get_by_token(&self, token: &str) -> Result<Session> {
        let mut conn = self.pool.acquire().await?;
        let session = query_as!(Session, r#"SELECT id, user_id, token, status as "status: _", name FROM session WHERE token = $1"#, token)
            .fetch_one(&mut *conn)
            .await
            .map_err(|err| match err.into() {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        Ok(session)
    }

    async fn session_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>> {
        todo!()
    }

    async fn session_delete(&self, id: SessionId) -> Result<()> {
        todo!()
    }
}
