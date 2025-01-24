use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};
use headers::{authorization::Bearer, Authorization, HeaderMapExt};
use types::{SessionToken, UserId};

use crate::{
    error::Error,
    types::{Session, SessionStatus},
    ServerState,
};

pub struct AuthRelaxed(pub Session);
pub struct Auth(pub Session, pub UserId);

impl FromRequestParts<Arc<ServerState>> for AuthRelaxed {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let auth: Authorization<Bearer> = parts
            .headers
            .typed_get()
            .ok_or_else(|| Error::MissingAuth)?;
        let session = s
            .data()
            .session_get_by_token(SessionToken(auth.token().to_string()))
            .await
            .map_err(|err| match err {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        Ok(Self(session))
    }
}

impl FromRequestParts<Arc<ServerState>> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthRelaxed(session) = AuthRelaxed::from_request_parts(parts, s).await?;
        match session.status {
            SessionStatus::Unauthorized => Err(Error::UnauthSession),
            SessionStatus::Authorized { user_id } => Ok(Self(session, user_id)),
            SessionStatus::Sudo { user_id } => Ok(Self(session, user_id)),
        }
    }
}
