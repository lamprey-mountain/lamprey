use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use common::v1::types::auth::AuthStatus;
use common::v1::types::auth::CaptchaChallenge;
use common::v1::types::auth::CaptchaResponse;
use common::v1::types::auth::PasswordExec;
use common::v1::types::auth::PasswordExecIdent;
use common::v1::types::auth::PasswordSet;
use common::v1::types::auth::TotpRecoveryCodes;
use common::v1::types::auth::TotpState;
use common::v1::types::auth::TotpStateWithSecret;
use common::v1::types::auth::TotpVerificationRequest;
use common::v1::types::email::EmailAddr;
use common::v1::types::util::Time;
use common::v1::types::MessageSync;
use common::v1::types::SessionStatus;
use common::v1::types::UserType;
use http::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use time::Duration;
use tracing::debug;
use url::Url;
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::types::DbUserCreate;
use crate::ServerState;

use crate::error::{Error, Result};

use super::util::Auth;
use super::util::AuthRelaxed;
use super::util::AuthSudo;
use super::util::AuthWithSession;

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
    params(("provider", description = "oauth provider")),
    tags = ["auth"],
    responses(
        (status = OK, body = OauthInitResponse, description = "ready"),
    )
)]
async fn auth_oauth_init(
    Path(provider): Path<String>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let url = s.services().oauth.create_url(&provider, session.id)?;
    Ok(Json(OauthInitResponse { url }))
}

/// Auth oauth redirect
#[utoipa::path(
    get,
    path = "/auth/oauth/{provider}/redirect",
    params(("provider", description = "oauth provider")),
    tags = ["auth"],
    responses(
        (status = OK, description = "success; responds with html + javascript"),
    )
)]
async fn auth_oauth_redirect(
    Path(_provider): Path<String>,
    Query(q): Query<OauthRedirectQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let (auth, session_id) = srv.oauth.exchange_code_for_token(q.state, q.code).await?;
    let dc = srv.oauth.discord_get_user(auth.access_token).await?;
    debug!("new discord user {:?}", dc);
    let data = s.data();
    let user_id = match data
        .auth_oauth_get_remote("discord".into(), dc.user.id.clone())
        .await
    {
        Ok(user_id) => user_id,
        Err(Error::NotFound) => {
            let user = data
                .user_create(DbUserCreate {
                    parent_id: None,
                    name: dc.user.global_name.unwrap_or(dc.user.username),
                    description: None,
                    user_type: UserType::Default,
                })
                .await?;
            data.auth_oauth_put("discord".into(), user.id, dc.user.id, true)
                .await?;
            user.id
        }
        Err(err) => return Err(err),
    };
    data.session_set_status(session_id, SessionStatus::Authorized { user_id })
        .await?;
    srv.sessions.invalidate(session_id).await;
    let session = srv.sessions.get(session_id).await?;
    s.broadcast(MessageSync::UpsertSession { session })?;
    Ok(Html(include_str!("../oauth.html")))
}

/// Auth oauth logout (TODO)
#[utoipa::path(
    post,
    path = "/auth/oauth/{provider}/logout",
    params(("provider", description = "oauth provider")),
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn auth_oauth_logout(
    Path(_provider): Path<String>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth oauth delete (TODO)
#[utoipa::path(
    delete,
    path = "/auth/oauth/{provider}",
    params(("provider", description = "oauth provider")),
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn auth_oauth_delete(
    Path(_provider): Path<String>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth oauth get (TODO)
#[utoipa::path(
    get,
    path = "/auth/oauth/{provider}",
    params(("provider", description = "oauth provider")),
    tags = ["auth"],
    responses((status = OK, description = "success"))
)]
async fn auth_oauth_get(
    Path(_provider): Path<String>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth email exec (TODO)
///
/// Send a "magic link" email to login
#[utoipa::path(
    post,
    path = "/auth/email/{addr}",
    params(("addr", description = "Email address")),
    tags = ["auth"],
    responses((status = ACCEPTED, description = "success")),
)]
async fn auth_email_exec(
    Path(_email): Path<EmailAddr>,
    AuthRelaxed(_session): AuthRelaxed,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth email reset (TODO)
///
/// Like exec, but the link also resets the password
#[utoipa::path(
    post,
    path = "/auth/email/{addr}/reset",
    params(("addr", description = "Email address")),
    tags = ["auth"],
    responses((status = ACCEPTED, description = "success")),
)]
async fn auth_email_reset(
    Path(_email): Path<EmailAddr>,
    AuthRelaxed(_session): AuthRelaxed,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp init (TODO)
#[utoipa::path(
    post,
    path = "/auth/totp/init",
    tags = ["auth"],
    responses((status = OK, body = TotpStateWithSecret, description = "success")),
)]
async fn auth_totp_init(
    AuthSudo(_auth_user_id): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp execute (TODO)
#[utoipa::path(
    post,
    path = "/auth/totp",
    tags = ["auth"],
    responses((status = OK, body = TotpState, description = "success")),
)]
async fn auth_totp_exec(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<TotpVerificationRequest>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp recovery codes get (TODO)
#[utoipa::path(
    get,
    path = "/auth/totp/recovery",
    tags = ["auth"],
    responses((status = OK, body = TotpRecoveryCodes, description = "success")),
)]
async fn auth_totp_recovery_get(
    AuthSudo(_auth_user_id): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp recovery codes rotate (TODO)
#[utoipa::path(
    post,
    path = "/auth/totp/recovery",
    tags = ["auth"],
    responses((status = OK, body = TotpRecoveryCodes, description = "success")),
)]
async fn auth_totp_recovery_rotate(
    AuthSudo(_auth_user_id): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp delete (TODO)
#[utoipa::path(
    delete,
    path = "/auth/totp",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_totp_delete(
    AuthSudo(_auth_user_id): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth password set
#[utoipa::path(
    put,
    path = "/auth/password",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_password_set(
    AuthSudo(auth_user_id): AuthSudo,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PasswordSet>,
) -> Result<impl IntoResponse> {
    let config = argon2::Config::default();
    let salt = {
        let mut salt = [0u8; 16];
        rand::fill(&mut salt);
        salt
    };
    let hash = argon2::hash_raw(json.password.as_bytes(), &salt, &config).unwrap();
    let data = s.data();
    data.auth_password_set(auth_user_id, &hash, &salt).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth password delete
#[utoipa::path(
    delete,
    path = "/auth/password",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_password_delete(
    AuthSudo(auth_user_id): AuthSudo,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    data.auth_password_delete(auth_user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth password exec
#[utoipa::path(
    post,
    path = "/auth/password",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_password_exec(
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PasswordExec>,
) -> Result<impl IntoResponse> {
    let user_id = match json.ident {
        PasswordExecIdent::UserId { user_id } => user_id,
        PasswordExecIdent::Email { email: _ } => todo!("lookup email stuff"),
    };
    let data = s.data();
    let config = argon2::Config::default();
    let (hash, salt) = data
        .auth_password_get(user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let valid = argon2::verify_raw(json.password.as_bytes(), &salt, &hash, &config)
        .map_err(|_| Error::NotFound)?;
    if valid {
        // TODO: allow entering sudo mode via password
        data.session_set_status(session.id, SessionStatus::Authorized { user_id })
            .await?;
        let srv = s.services();
        srv.sessions.invalidate(session.id).await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound)
    }
}

/// Auth status (TODO)
#[utoipa::path(
    get,
    path = "/auth",
    tags = ["auth"],
    responses((status = OK, body = AuthStatus, description = "success")),
)]
async fn auth_status(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth captcha init (TODO)
#[utoipa::path(
    post,
    path = "/auth/captcha/init",
    tags = ["auth"],
    responses((status = OK, body = CaptchaChallenge, description = "success")),
)]
async fn auth_captcha_init(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth captcha submit (TODO)
#[utoipa::path(
    post,
    path = "/auth/captcha/submit",
    tags = ["auth"],
    responses(
        (status = NO_CONTENT, description = "captcha ok"),
        (status = UNAUTHORIZED, description = "captcha failure"),
    ),
)]
async fn auth_captcha_submit(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<CaptchaResponse>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth sudo (TEMP)
///
/// for debugging
#[utoipa::path(
    post,
    path = "/auth/_sudo",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "ok")),
)]
async fn auth_sudo(
    AuthWithSession(session, auth_user_id): AuthWithSession,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    s.data()
        .session_set_status(
            session.id,
            SessionStatus::Sudo {
                user_id: auth_user_id,
                expires_at: Time::now_utc().saturating_add(Duration::minutes(5)).into(),
            },
        )
        .await?;
    s.services().sessions.invalidate(session.id).await;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(auth_oauth_init))
        .routes(routes!(auth_oauth_redirect))
        .routes(routes!(auth_oauth_logout))
        .routes(routes!(auth_oauth_delete))
        .routes(routes!(auth_oauth_get))
        .routes(routes!(auth_email_exec))
        .routes(routes!(auth_email_reset))
        .routes(routes!(auth_totp_init))
        .routes(routes!(auth_totp_exec))
        .routes(routes!(auth_totp_delete))
        .routes(routes!(auth_totp_recovery_get))
        .routes(routes!(auth_totp_recovery_rotate))
        .routes(routes!(auth_password_set))
        .routes(routes!(auth_password_delete))
        .routes(routes!(auth_password_exec))
        .routes(routes!(auth_captcha_init))
        .routes(routes!(auth_captcha_submit))
        .routes(routes!(auth_status))
        .routes(routes!(auth_sudo))
}
