use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use types::SessionStatus;
use url::Url;
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::types::UserCreate;
use crate::ServerState;

use crate::error::{Error, Result};

use super::util::AuthRelaxed;

#[derive(Debug, Deserialize, IntoParams)]
pub struct OauthRedirectQuery {
    state: Uuid,
    code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OauthInitResponse {
    url: Url,
}

/// Auth oauth init
#[utoipa::path(
    post,
    path = "/auth/oauth/{provider}",
    tags = ["session"],
    responses(
        (status = OK, body = OauthInitResponse, description = "ready"),
    )
)]
pub async fn auth_oauth_init(
    Path(provider): Path<String>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let url = s.services().oauth_create_url(&provider, session.id)?;
    Ok(Json(OauthInitResponse { url }))
}

/// Auth oauth redirect
#[utoipa::path(
    get,
    path = "/auth/oauth/{provider}/redirect",
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn auth_oauth_redirect(
    Path(_provider): Path<String>,
    Query(q): Query<OauthRedirectQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let (auth, session_id) = srv.oauth_exchange_code_for_token(q.state, q.code).await?;
    let dc = srv.discord_get_user(auth.access_token).await?;
    debug!("new discord user {:?}", dc);
    let data = s.data();
    let user_id = match data
        .auth_oauth_get_remote("discord".into(), dc.user.id.clone())
        .await
    {
        Ok(user_id) => user_id,
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
            data.auth_oauth_put("discord".into(), user.id, dc.user.id)
                .await?;
            user.id
        }
        Err(err) => return Err(err),
    };
    data.session_set_status(session_id, SessionStatus::Authorized { user_id })
        .await?;
    let session = data.session_get(session_id).await?;
    s.broadcast(types::MessageSync::UpsertSession { session })?;
    Ok(Html(include_str!("../oauth.html")))
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
        // .routes(routes!(auth_discord_init))
        // .routes(routes!(auth_discord_redirect))
        .routes(routes!(auth_oauth_init))
        .routes(routes!(auth_oauth_redirect))
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

// planning
// enum AuthAction {
//     OauthStart { provider: String },
//     // -> Authorized
//     OauthFinish { state: Uuid, code: String },
//     // -> Authorized
//     EmailPassword { email: String, password: String },
//     // -> Authorized
//     EmailLink { email: String },
//     // -> Sudo
//     Totp { code: String },
//     // -> Sudo
//     SudoPassword { password: String },
//     Captcha { code: String },
// }

// // requires sudo mode; cannot change auth in a way that locks you out of sudo mode
// enum AuthUpdate {
//     LinkTotp,                   // -> code
//     LinkEmail { addr: String }, // -> send verification email
//     LinkPassword { pass: String },
//     UnlinkOauth { provider: String },
//     UnlinkTotp {},
//     UnlinkEmail {},
//     UnlinkPassword {},
// }
