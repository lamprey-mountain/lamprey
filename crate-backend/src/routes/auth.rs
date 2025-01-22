use std::sync::Arc;

use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::{extract::State, Json};
use base64::Engine;
use headers::authorization::Credentials;
use headers::HeaderMapExt;
use serde::{Deserialize, Serialize};
use tracing::debug;
use types::{SessionCreate, SessionStatus};
use url::Url;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::types::UserCreate;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

// const validStates = new Set();

/// TEMP: Auth discord init
///
/// This will be replaced later with a more robust system
// #[utoipa::path(
//     get,
//     path = "/session/{session_id}/auth/discord",
//     params(
//         ("session_id", description = "Session id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = FOUND, description = "success"),
//     )
// )]
#[utoipa::path(
    get,
    path = "/auth/discord",
    tags = ["session"],
    responses(
        (status = FOUND, description = "success"),
    )
)]
pub async fn auth_discord_init(State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let url = s.services().oauth_create_url()?;
    let mut headers = HeaderMap::new();
    let redir = headers::HeaderValue::from_str(&url).expect("invalid location header?");
    headers.insert("location", redir);
    Ok((StatusCode::FOUND, headers))
}

#[derive(Debug, Deserialize)]
pub struct Oauth2RedirectQuery {
    state: Uuid,
    code: String,
}

/// Auth discord redirect
// #[utoipa::path(
//     get,
//     path = "/session/{session_id}/auth/discord/redirect",
//     params(
//         ("session_id", description = "Session id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
#[utoipa::path(
    get,
    path = "/auth/discord/redirect",
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn auth_discord_redirect(
    Query(q): Query<Oauth2RedirectQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    if s.valid_oauth2_states.remove(&q.state).is_none() {
        return Err(Error::BadStatic("invalid or expired state"));
    }
    let srv = s.services();
    let auth = srv.oauth_exchange_code_for_token(q.code).await?;
    let dc = srv.oauth_get_user(auth.access_token).await?;
    debug!("new discord user {:?}", dc);
    let data = s.data();
    let user = match s.data().temp_user_get_by_discord_id(dc.user.id.clone()).await {
        Ok(user) => user,
        Err(Error::NotFound) => {
            let user = data
                .user_create(UserCreate {
                    parent_id: None,
                    name: dc.user.global_name.unwrap_or(dc.user.username),
                    description: None,
                    status: None,
                    is_bot: false,
                    is_alias: false,
                    is_system: false,
                })
                .await?;
            data.temp_user_set_discord_id(user.id, dc.user.id).await?;
            user
        }
        Err(err) => return Err(err),
    };
    let s = data.session_create(user.id, None).await?;
    data.session_set_status(s.id, SessionStatus::Authorized)
        .await?;
    Ok(Html(format!(
        r#"
         <pre>Success! You should be redirected; if not, click <a href="/">here</a></pre>
         <script>
           localStorage.setItem("token", "{}");
           localStorage.setItem("user_id", "{}");
           location.href = "/";
         </script>
    "#, s.token, user.id)))
}

// /// Auth discord logout
// #[utoipa::path(
//     delete,
//     path = "/session/{session_id}/auth/discord",
//     params(
//         ("session_id", description = "Session id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn auth_discord_logout(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<impl IntoResponse> {
//     todo!()
// }

// /// Auth discord get
// #[utoipa::path(
//     get,
//     path = "/users/{user_id}/auth/discord",
//     params(
//         ("session_id", description = "Session id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn auth_discord_get(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<impl IntoResponse> {
//     todo!()
// }

// /// Auth discord delete
// ///
// /// Delete the link between discord and this user
// #[utoipa::path(
//     delete,
//     path = "/users/{user_id}/auth/discord",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn auth_discord_delete(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<impl IntoResponse> {
//     todo!()
// }

// /// Auth email set
// #[utoipa::path(
//     put,
//     path = "/users/{user_id}/auth/email",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = CREATED, description = "success"),
//         (status = OK, description = "already exists"),
//     )
// )]
// pub async fn auth_email_set(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<impl IntoResponse> {
//     todo!()
// }

// /// Auth email get
// #[utoipa::path(
//     get,
//     path = "/users/{user_id}/auth/email",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//         (status = NOT_FOUND, description = "doesn't exist"),
//     )
// )]
// pub async fn auth_email_set(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<impl IntoResponse> {
//     todo!()
// }

// /// Auth email delete
// #[utoipa::path(
//     delete,
//     path = "/users/{user_id}/auth/email",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = CREATED, description = "success"),
//         (status = OK, description = "already exists"),
//     )
// )]
// pub async fn auth_email_delete(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<impl IntoResponse> {
//     todo!()
// }

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(auth_discord_init))
        .routes(routes!(auth_discord_redirect))
    // .routes(routes!(auth_discord_logout))
    // .routes(routes!(auth_discord_delete))
    // .routes(routes!(auth_discord_get))
    // .routes(routes!(auth_email_exec))
    // .routes(routes!(auth_email_set))
    // .routes(routes!(auth_email_get))
    // .routes(routes!(auth_email_delete))
    // .routes(routes!(auth_totp_set))
    // .routes(routes!(auth_totp_exec))
}
