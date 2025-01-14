use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{
    error::Error,
    types::{Session, SessionStatus},
    ServerState,
};

// pub struct DatabaseConnection(pub sqlx::pool::PoolConnection<sqlx::Postgres>);

pub struct Auth(pub Session);

// impl FromRequestParts<ServerState> for DatabaseConnection {
//     type Rejection = Error;

//     async fn from_request_parts(
//         _parts: &mut Parts,
//         state: &ServerState,
//     ) -> Result<Self, Self::Rejection> {
//         let conn = state.pool.acquire().await?;
//         Ok(Self(conn))
//     }
// }

impl FromRequestParts<ServerState> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get("authorization")
            .ok_or(Error::MissingAuth)?
            .to_str()?
            .to_string();
        let session = s.data().session_get_by_token(&auth).await?;
        if session.status == SessionStatus::Unauthorized {
            return Err(Error::UnauthSession);
        }
        Ok(Self(session))
    }
}
