use axum::{extract::FromRequestParts, http::request::Parts};
use sqlx::query_as;

use crate::{error::Error, types::{Session, SessionStatus}, ServerState};

pub struct DatabaseConnection(pub sqlx::pool::PoolConnection<sqlx::Postgres>);

pub struct Auth(pub Session);

impl FromRequestParts<ServerState> for DatabaseConnection {
    type Rejection = Error;

    async fn from_request_parts(_parts: &mut Parts, state: &ServerState) -> Result<Self, Self::Rejection> {
        let conn = state.pool.acquire().await?;
        Ok(Self(conn))
    }
}

impl FromRequestParts<ServerState> for Auth {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &ServerState) -> Result<Self, Self::Rejection> {
        let auth = parts.headers.get("authorization").ok_or(Error::MissingAuthHeader)?.to_str()?.to_string();
        let mut conn = DatabaseConnection::from_request_parts(parts, state).await?;
        let session = query_as!(Session, r#"SELECT id, user_id, token, status AS "status: _", name FROM session WHERE token = $1"#, auth)
            .fetch_one(&mut *conn.0)
            .await?;
        if session.status == SessionStatus::Unauthorized {
            return Err(Error::UnauthSession);
        }
        Ok(Self(session))
    }
}
