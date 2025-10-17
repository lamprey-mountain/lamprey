use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use common::v1::types::auth::{
    AuthState, CaptchaChallenge, CaptchaResponse, PasswordExec, PasswordExecIdent, PasswordSet,
    TotpRecoveryCodes, TotpState, TotpStateWithSecret, TotpVerificationRequest,
    WebauthnAuthenticator, WebauthnChallenge, WebauthnFinish, WebauthnPatch,
};
use common::v1::types::email::EmailAddr;
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogChange, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, SessionStatus,
    UserId,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use time::Duration;
use tracing::debug;
use url::Url;
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::routes::util::AuthSudoWithSession;
use crate::routes::util::HeaderReason;
use crate::types::DbUserCreate;
use crate::types::EmailPurpose;
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
    Path(provider): Path<String>,
    Query(q): Query<OauthRedirectQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    match provider.as_str() {
        "discord" => {
            let (auth, session_id) = srv.oauth.exchange_code_for_token(q.state, q.code).await?;
            let u = srv.oauth.discord_get_user(auth.access_token).await?;
            debug!("new discord user {:?}", u);
            let user_id = match data
                .auth_oauth_get_remote("discord".into(), u.user.id.clone())
                .await
            {
                Ok(user_id) => user_id,
                Err(Error::NotFound) => {
                    let user = data
                        .user_create(DbUserCreate {
                            id: None,
                            parent_id: None,
                            name: u.user.global_name.unwrap_or(u.user.username),
                            description: None,
                            puppet: None,
                            registered_at: None,
                            system: false,
                        })
                        .await?;
                    data.auth_oauth_put("discord".into(), user.id, u.user.id, true)
                        .await?;
                    user.id
                }
                Err(err) => return Err(err),
            };
            data.session_set_status(session_id, SessionStatus::Authorized { user_id })
                .await?;
            srv.sessions.invalidate(session_id).await;
            let session = srv.sessions.get(session_id).await?;
            s.broadcast(MessageSync::SessionCreate {
                session: session.clone(),
            })?;
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: user_id.into_inner().into(),
                user_id,
                session_id: Some(session_id),
                reason: None,
                ty: AuditLogEntryType::SessionLogin {
                    user_id,
                    session_id,
                },
            })
            .await?;
            Ok(Html(include_str!("../oauth.html")))
        }
        "github" => {
            let (auth, session_id) = srv.oauth.exchange_code_for_token(q.state, q.code).await?;
            let u = srv.oauth.github_get_user(auth.access_token).await?;
            debug!("new github user {:?}", u);
            let user_id = match data
                .auth_oauth_get_remote("github".into(), u.id.to_string())
                .await
            {
                Ok(user_id) => user_id,
                Err(Error::NotFound) => {
                    let user = data
                        .user_create(DbUserCreate {
                            id: None,
                            parent_id: None,
                            name: u.name.unwrap_or(u.login),
                            description: u.bio,
                            puppet: None,
                            registered_at: None,
                            system: false,
                        })
                        .await?;
                    data.auth_oauth_put("github".into(), user.id, u.id.to_string(), true)
                        .await?;
                    user.id
                }
                Err(err) => return Err(err),
            };
            data.session_set_status(session_id, SessionStatus::Authorized { user_id })
                .await?;
            srv.sessions.invalidate(session_id).await;
            let session = srv.sessions.get(session_id).await?;
            s.broadcast(MessageSync::SessionCreate {
                session: session.clone(),
            })?;
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: user_id.into_inner().into(),
                user_id,
                session_id: Some(session_id),
                reason: None,
                ty: AuditLogEntryType::SessionLogin {
                    user_id,
                    session_id,
                },
            })
            .await?;
            Ok(Html(include_str!("../oauth.html")))
        }
        _ => return Err(Error::Unimplemented),
    }
}

/// Auth oauth delete
///
/// Remove an oauth provider. You will no longer be able to authenticate via
/// this provider after this endpoint is called.
#[utoipa::path(
    delete,
    path = "/auth/oauth/{provider}",
    params(("provider", description = "oauth provider")),
    tags = ["auth", "badge.sudo"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn auth_oauth_delete(
    Path(provider): Path<String>,
    AuthSudoWithSession(session, auth_user): AuthSudoWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let start_state = fetch_auth_state(&s, auth_user.id).await?;
    let data = s.data();
    data.auth_oauth_delete(provider, auth_user.id).await?;
    let end_state = fetch_auth_state(&s, auth_user.id).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate {
            changes: Changes::new()
                .change(
                    "oauth_providers",
                    &start_state.oauth_providers,
                    &end_state.oauth_providers,
                )
                .build(),
        },
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth email exec
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
    Path(email): Path<EmailAddr>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let code = Uuid::new_v4().to_string();
    d.auth_email_create(code.clone(), email.clone(), session.id, EmailPurpose::Authn)
        .await?;
    let mut url = s.config.html_url.join("email-auth")?;
    url.set_query(Some(&format!("code={code}")));
    let message = format!(
        "click this link to login: {url}\n\nif you didn't request this, ignore this email."
    );
    srv.email
        .send(email, "Login to lamprey".to_string(), message, None)
        .await?;
    Ok(())
}

/// Auth email reset
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
    Path(email): Path<EmailAddr>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let code = Uuid::new_v4().to_string();
    d.auth_email_create(code.clone(), email.clone(), session.id, EmailPurpose::Reset)
        .await?;
    let mut url = s.config.html_url.join("email-auth")?;
    url.set_query(Some(&format!("code={code}")));
    let message = format!("click this link to reset password: {url}\n\nif you didn't request this, ignore this email.");
    srv.email
        .send(email, "Lamprey password reset".to_string(), message, None)
        .await?;
    Ok(())
}

/// Auth email complete
///
/// Consume an email auth code to log in
#[utoipa::path(
    post,
    path = "/auth/email/{addr}/complete",
    params(("addr", description = "Email address")),
    tags = ["auth"],
    responses((status = ACCEPTED, description = "success")),
)]
async fn auth_email_complete(
    Path(email): Path<EmailAddr>,
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AuthEmailComplete>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let (req_addr, req_session, purpose) = d.auth_email_use(json.code).await?;

    if req_addr != email {
        debug!("wrong email");
        return Err(Error::BadStatic("invalid or expired code"));
    }

    if req_session != session.id {
        debug!("wrong session");
        return Err(Error::BadStatic("invalid or expired code"));
    }

    if session.status != SessionStatus::Unauthorized {
        debug!("already authenticated");
        return Err(Error::BadStatic("invalid or expired code"));
    }

    let user_id = d.user_email_lookup(&email).await?;
    let status = match purpose {
        EmailPurpose::Authn => SessionStatus::Authorized { user_id },

        // TODO: there's probably a better way of implementing password resets than directly entering sudo mode
        // maybe some "semi sudo mode" that only allows changing password?
        // this isn't *that* bad though and chances are if someone reset their password they may want to do other stuff too
        EmailPurpose::Reset => SessionStatus::Sudo {
            user_id,
            sudo_expires_at: Time::now_utc().saturating_add(Duration::minutes(5)).into(),
        },
    };
    d.session_set_status(session.id, status.clone()).await?;
    srv.sessions.invalidate(session.id).await;
    let session = srv.sessions.get(session.id).await?;
    s.broadcast(MessageSync::SessionCreate {
        session: session.clone(),
    })?;

    match purpose {
        EmailPurpose::Authn => {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: user_id.into_inner().into(),
                user_id,
                session_id: Some(session.id),
                reason,
                ty: AuditLogEntryType::SessionLogin {
                    user_id,
                    session_id: session.id,
                },
            })
            .await?;
        }
        EmailPurpose::Reset => {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: user_id.into_inner().into(),
                user_id,
                session_id: Some(session.id),
                reason,
                ty: AuditLogEntryType::AuthSudo {
                    session_id: session.id,
                },
            })
            .await?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize, ToSchema)]
struct AuthEmailComplete {
    code: String,
}

/// Auth totp init (TODO)
#[utoipa::path(
    post,
    path = "/auth/totp/init",
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = TotpStateWithSecret, description = "success")),
)]
async fn auth_totp_init(
    AuthSudo(_auth_user): AuthSudo,
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
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<TotpVerificationRequest>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp recovery codes get (TODO)
#[utoipa::path(
    get,
    path = "/auth/totp/recovery",
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = TotpRecoveryCodes, description = "success")),
)]
async fn auth_totp_recovery_get(
    AuthSudo(_auth_user): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp recovery codes rotate (TODO)
#[utoipa::path(
    post,
    path = "/auth/totp/recovery",
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = TotpRecoveryCodes, description = "success")),
)]
async fn auth_totp_recovery_rotate(
    AuthSudo(_auth_user): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth totp delete (TODO)
#[utoipa::path(
    delete,
    path = "/auth/totp",
    tags = ["auth", "badge.sudo"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_totp_delete(
    AuthSudo(_auth_user): AuthSudo,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth password set
#[utoipa::path(
    put,
    path = "/auth/password",
    tags = ["auth", "badge.sudo"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_password_set(
    AuthSudoWithSession(session, auth_user): AuthSudoWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PasswordSet>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let start_has_password = data.auth_password_get(auth_user.id).await?.is_some();

    let config = argon2::Config::default();
    let salt = {
        let mut salt = [0u8; 16];
        rand::fill(&mut salt);
        salt
    };
    let hash = argon2::hash_raw(json.password.as_bytes(), &salt, &config).unwrap();
    data.auth_password_set(auth_user.id, &hash, &salt).await?;

    let end_has_password = data.auth_password_get(auth_user.id).await?.is_some();

    let mut changes = Vec::new();
    changes.push(AuditLogChange {
        key: "has_password".into(),
        old: serde_json::to_value(start_has_password).unwrap(),
        new: serde_json::to_value(end_has_password).unwrap(),
    });

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate { changes },
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth password delete
#[utoipa::path(
    delete,
    path = "/auth/password",
    tags = ["auth", "badge.sudo"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_password_delete(
    AuthSudoWithSession(session, auth_user): AuthSudoWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let has_password = data.auth_password_get(auth_user.id).await?.is_some();
    if !has_password {
        return Ok(());
    }

    data.auth_password_delete(auth_user.id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate {
            changes: Changes::new()
                .change("has_password", &has_password, &false)
                .build(),
        },
    })
    .await?;

    Ok(())
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
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PasswordExec>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let user_id = match json.ident {
        PasswordExecIdent::UserId { user_id } => user_id,
        PasswordExecIdent::Email { email } => data.user_email_lookup(&email).await?,
    };
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
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: user_id.into_inner().into(),
            user_id,
            session_id: Some(session.id),
            reason,
            ty: AuditLogEntryType::SessionLogin {
                user_id,
                session_id: session.id,
            },
        })
        .await?;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound)
    }
}

/// Auth state
///
/// Get the available auth methods for this user
#[utoipa::path(
    get,
    path = "/auth",
    tags = ["auth"],
    responses((status = OK, body = AuthState, description = "success")),
)]
async fn auth_state(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let auth_state = fetch_auth_state(&s, auth_user.id).await?;
    Ok(Json(auth_state))
}

pub async fn fetch_auth_state(s: &ServerState, user_id: UserId) -> Result<AuthState> {
    let data = s.data();
    let oauth_providers = data.auth_oauth_get_all(user_id).await?;
    let email = data.user_email_list(user_id).await?;
    let password = data.auth_password_get(user_id).await?;
    let auth_state = AuthState {
        has_email: email.iter().any(|e| e.is_verified && e.is_primary),
        has_totp: false, // totp not implemented yet
        has_password: password.is_some(),
        oauth_providers,
        authenticators: vec![], // webauthn not implemented yet
    };
    Ok(auth_state)
}

/// Auth captcha init (TODO)
#[utoipa::path(
    post,
    path = "/auth/captcha/init",
    tags = ["auth"],
    responses((status = OK, body = CaptchaChallenge, description = "success")),
)]
async fn auth_captcha_init(
    Auth(_auth_user): Auth,
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
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<CaptchaResponse>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Auth webauthn init (TODO)
#[utoipa::path(
    get,
    path = "/auth/webauthn/init",
    tags = ["auth"],
    responses((status = OK, body = WebauthnChallenge, description = "webauthn challenge")),
)]
async fn auth_webauthn_init(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Auth webauthn exec (TODO)
///
/// Register a new authenticator or login with one
#[utoipa::path(
    post,
    path = "/auth/webauthn/exec",
    tags = ["auth"],
    responses(
        (status = NO_CONTENT, description = "success"),
    ),
)]
async fn auth_webauthn_exec(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<WebauthnFinish>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Auth webauthn patch (TODO)
#[utoipa::path(
    patch,
    path = "/auth/webauthn/authenticator/{authenticator_id}",
    tags = ["auth"],
    responses(
        (status = OK, body = WebauthnAuthenticator, description = "success"),
    ),
)]
async fn auth_webauthn_patch(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_authenticator_id): Path<Uuid>,
    Json(_json): Json<WebauthnPatch>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Auth webauthn delete (TODO)
#[utoipa::path(
    delete,
    path = "/auth/webauthn/authenticator/{authenticator_id}",
    tags = ["auth"],
    responses(
        (status = NO_CONTENT, description = "success"),
    ),
)]
async fn auth_webauthn_delete(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_authenticator_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Auth sudo (TEMP)
///
/// instantly upgrade to sudo mode; this is intended for debugging
#[utoipa::path(
    post,
    path = "/auth/_sudo",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "ok")),
)]
async fn auth_sudo(
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    s.data()
        .session_set_status(
            session.id,
            SessionStatus::Sudo {
                user_id: auth_user.id,
                sudo_expires_at: Time::now_utc().saturating_add(Duration::minutes(5)).into(),
            },
        )
        .await?;
    s.services().sessions.invalidate(session.id).await;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::AuthSudo {
            session_id: session.id,
        },
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(auth_oauth_init))
        .routes(routes!(auth_oauth_redirect))
        .routes(routes!(auth_oauth_delete))
        .routes(routes!(auth_email_exec))
        .routes(routes!(auth_email_reset))
        .routes(routes!(auth_email_complete))
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
        .routes(routes!(auth_webauthn_init))
        .routes(routes!(auth_webauthn_exec))
        .routes(routes!(auth_webauthn_patch))
        .routes(routes!(auth_webauthn_delete))
        .routes(routes!(auth_state))
        .routes(routes!(auth_sudo))
}
