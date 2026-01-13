use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use common::v1::types::auth::TotpInit;
use common::v1::types::auth::TotpVerificationRequest;
use common::v1::types::auth::{
    AuthState, CaptchaChallenge, CaptchaResponse, PasswordExec, PasswordExecIdent, PasswordSet,
    TotpRecoveryCode, TotpRecoveryCodes, WebauthnAuthenticator, WebauthnChallenge, WebauthnFinish,
    WebauthnPatch,
};
use common::v1::types::email::EmailAddr;
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogChange, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, RoomMemberPut,
    SessionStatus, UserId, SERVER_ROOM_ID,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Secret as TotpSecret, TOTP as Totp};
use tracing::debug;
use url::Url;
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::routes::util::{Auth, HeaderReason};
use crate::types::DbUserCreate;
use crate::types::EmailPurpose;
use crate::ServerState;

use crate::error::{Error, Result};

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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let url = s.services().oauth.create_url(&provider, auth.session.id)?;
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
    let provider_config = s
        .config
        .oauth_provider
        .get(&provider)
        .ok_or(Error::Unimplemented)?;
    match provider.as_str() {
        "discord" => {
            let (auth, session_id) = srv.oauth.exchange_code_for_token(q.state, q.code).await?;
            let u = srv.oauth.discord_get_user(auth.access_token).await?;
            debug!("new discord user {:?}", u);
            let user_id = match data
                .auth_oauth_get_remote("discord".into(), u.user.id.clone())
                .await
            {
                Ok(user_id) => {
                    let user = srv.users.get(user_id, None).await?;
                    if provider_config.autoregister && user.registered_at.is_none() {
                        data.user_set_registered(user.id, Some(Time::now_utc()), None)
                            .await?;
                        data.room_member_put(
                            SERVER_ROOM_ID,
                            user.id,
                            None,
                            RoomMemberPut::default(),
                        )
                        .await?;
                        srv.users.invalidate(user.id).await;
                        let updated_user = srv.users.get(user.id, None).await?;
                        s.audit_log_append(AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: Some("oauth_autoregister".to_string()),
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                        })
                        .await?;
                        s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
                    }
                    user_id
                }
                Err(Error::NotFound) => {
                    let registered_at = if provider_config.autoregister {
                        Some(Time::now_utc())
                    } else {
                        None
                    };
                    let user = data
                        .user_create(DbUserCreate {
                            id: None,
                            parent_id: None,
                            name: u.user.global_name.unwrap_or(u.user.username),
                            description: None,
                            puppet: None,
                            registered_at,
                            system: false,
                        })
                        .await?;
                    if provider_config.autoregister {
                        data.room_member_put(
                            SERVER_ROOM_ID,
                            user.id,
                            None,
                            RoomMemberPut::default(),
                        )
                        .await?;
                        s.audit_log_append(AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: Some("oauth_autoregister".to_string()),
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                        })
                        .await?;
                    }
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
                Ok(user_id) => {
                    let user = srv.users.get(user_id, None).await?;
                    if provider_config.autoregister && user.registered_at.is_none() {
                        data.user_set_registered(user.id, Some(Time::now_utc()), None)
                            .await?;
                        data.room_member_put(
                            SERVER_ROOM_ID,
                            user.id,
                            None,
                            RoomMemberPut::default(),
                        )
                        .await?;
                        srv.users.invalidate(user.id).await;
                        let updated_user = srv.users.get(user.id, None).await?;
                        s.audit_log_append(AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: Some("oauth_autoregister".to_string()),
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                        })
                        .await?;
                        s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
                    }
                    user_id
                }
                Err(Error::NotFound) => {
                    let registered_at = if provider_config.autoregister {
                        Some(Time::now_utc())
                    } else {
                        None
                    };
                    let user = data
                        .user_create(DbUserCreate {
                            id: None,
                            parent_id: None,
                            name: u.name.unwrap_or(u.login),
                            description: u.bio,
                            puppet: None,
                            registered_at,
                            system: false,
                        })
                        .await?;
                    if provider_config.autoregister {
                        data.room_member_put(
                            SERVER_ROOM_ID,
                            user.id,
                            None,
                            RoomMemberPut::default(),
                        )
                        .await?;
                        s.audit_log_append(AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: Some("oauth_autoregister".to_string()),
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                        })
                        .await?;
                    }
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let start_state = fetch_auth_state(&s, auth.user.id).await?;
    let data = s.data();
    data.auth_oauth_delete(provider, auth.user.id).await?;
    let end_state = fetch_auth_state(&s, auth.user.id).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let code = Uuid::new_v4().to_string();
    d.auth_email_create(
        code.clone(),
        email.clone(),
        auth.session.id,
        EmailPurpose::Authn,
    )
    .await?;
    let mut url = s.config.html_url.join("email-auth")?;
    url.set_query(Some(&format!("code={code}")));
    let message = format!(
        "click this link to login: {url}\n\nif you didn't request this, ignore this email."
    );
    srv.email
        .send(email, "Login to lamprey".to_string(), message, None)
        .await?;
    Ok(StatusCode::NO_CONTENT)
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let code = Uuid::new_v4().to_string();
    d.auth_email_create(
        code.clone(),
        email.clone(),
        auth.session.id,
        EmailPurpose::Reset,
    )
    .await?;
    let mut url = s.config.html_url.join("email-auth")?;
    url.set_query(Some(&format!("code={code}")));
    let message = format!("click this link to reset password: {url}\n\nif you didn't request this, ignore this email.");
    srv.email
        .send(email, "Lamprey password reset".to_string(), message, None)
        .await?;
    Ok(StatusCode::NO_CONTENT)
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
    auth: Auth,
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

    if req_session != auth.session.id {
        debug!("wrong session");
        return Err(Error::BadStatic("invalid or expired code"));
    }

    if auth.session.status != SessionStatus::Unauthorized {
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
    d.session_set_status(auth.session.id, status.clone())
        .await?;
    srv.sessions.invalidate(auth.session.id).await;
    let session = srv.sessions.get(auth.session.id).await?;
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
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, ToSchema)]
struct AuthEmailComplete {
    code: String,
}

/// Auth totp init
///
/// Begin totp registration by generating a secret
#[utoipa::path(
    post,
    path = "/auth/totp/init",
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = TotpInit, description = "success")),
)]
async fn auth_totp_init(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;

    let mut secret_bytes = [0u8; 20];
    rand::fill(&mut secret_bytes);

    let secret = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret_bytes);

    s.data()
        .auth_totp_set(auth.user.id, Some(secret.clone()), false)
        .await?;

    Ok(Json(TotpInit { secret }))
}

/// Auth totp complete
///
/// Complete the totp registration process
#[utoipa::path(
    post,
    path = "/auth/totp/complete",
    request_body = TotpVerificationRequest,
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = AuthState, description = "success")),
)]
async fn auth_totp_complete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<TotpVerificationRequest>,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let (secret, enabled) = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .ok_or(Error::BadStatic("totp not initialized"))?;

    if enabled {
        return Err(Error::BadStatic("totp already enabled"));
    }

    let secret_bytes = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret)
        .ok_or_else(|| Error::Internal("failed to decode totp secret".to_owned()))?;

    let totp = Totp::new(
        TotpAlgorithm::SHA1,
        6,
        1,
        30,
        TotpSecret::Raw(secret_bytes).to_bytes().unwrap(),
    )
    .map_err(|e| {
        tracing::error!("failed to create totp: {}", e);
        Error::Internal("failed to create totp".to_owned())
    })?;

    if !totp.check_current(&json.code).unwrap_or(false) {
        return Err(Error::BadStatic("invalid totp code"));
    }

    s.data()
        .auth_totp_set(auth.user.id, Some(secret), true)
        .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate {
            changes: Changes::new().change("has_totp", &false, &true).build(),
        },
    })
    .await?;

    let auth_state = fetch_auth_state(&s, auth.user.id).await?;

    Ok(Json(auth_state))
}

/// Auth totp execute
#[utoipa::path(
    post,
    path = "/auth/totp",
    request_body = TotpVerificationRequest,
    tags = ["auth"],
    responses((status = OK, body = AuthState, description = "success")),
)]
async fn auth_totp_exec(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<TotpVerificationRequest>,
) -> Result<impl IntoResponse> {
    let (secret, enabled) = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .ok_or(Error::BadStatic("totp not enabled"))?;

    if !enabled {
        return Err(Error::BadStatic("totp not enabled"));
    }

    let secret_bytes = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret)
        .ok_or_else(|| Error::Internal("failed to decode totp secret".to_owned()))?;

    let totp = Totp::new(
        TotpAlgorithm::SHA1,
        6,
        1,
        30,
        TotpSecret::Raw(secret_bytes).to_bytes().unwrap(),
    )
    .map_err(|e| {
        tracing::error!("failed to create totp: {}", e);
        Error::Internal("failed to create totp".to_owned())
    })?;

    if !totp.check_current(&json.code).unwrap_or(false) {
        return Err(Error::BadStatic("invalid totp code"));
    }

    s.data()
        .session_set_status(
            auth.session.id,
            SessionStatus::Sudo {
                user_id: auth.user.id,
                sudo_expires_at: Time::now_utc().saturating_add(Duration::minutes(5)).into(),
            },
        )
        .await?;
    s.services().sessions.invalidate(auth.session.id).await;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthSudo {
            session_id: auth.session.id,
        },
    })
    .await?;

    let auth_state = fetch_auth_state(&s, auth.user.id).await?;

    Ok(Json(auth_state))
}

/// Auth totp recovery codes get
#[utoipa::path(
    get,
    path = "/auth/totp/recovery",
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = TotpRecoveryCodes, description = "success")),
)]
async fn auth_totp_recovery_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<TotpRecoveryCodes>> {
    auth.ensure_sudo()?;
    let codes_with_used_at = s.data().auth_totp_recovery_get_all(auth.user.id).await?;
    let recovery_codes = codes_with_used_at
        .into_iter()
        .map(|(code, used_at)| TotpRecoveryCode { code, used_at })
        .collect();
    Ok(Json(TotpRecoveryCodes {
        codes: recovery_codes,
    }))
}

/// Auth totp recovery codes rotate
#[utoipa::path(
    post,
    path = "/auth/totp/recovery",
    tags = ["auth", "badge.sudo"],
    responses((status = OK, body = TotpRecoveryCodes, description = "success")),
)]
async fn auth_totp_recovery_rotate(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<Json<TotpRecoveryCodes>> {
    auth.ensure_sudo()?;
    let mut codes = Vec::with_capacity(5);
    for _ in 0..5 {
        codes.push(Uuid::new_v4().to_string());
    }

    s.data()
        .auth_totp_recovery_generate(auth.user.id, &codes)
        .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate {
            changes: vec![AuditLogChange {
                key: "totp_recovery_codes_rotated".into(),
                old: Value::Null,
                new: Value::Null,
            }],
        },
    })
    .await?;

    let recovery_codes = codes
        .into_iter()
        .map(|code| TotpRecoveryCode {
            code,
            used_at: None,
        })
        .collect();
    Ok(Json(TotpRecoveryCodes {
        codes: recovery_codes,
    }))
}

/// Auth totp delete
#[utoipa::path(
    delete,
    path = "/auth/totp",
    tags = ["auth", "badge.sudo"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_totp_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let had_totp = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .map(|(_, enabled)| enabled)
        .unwrap_or(false);

    if !had_totp {
        return Ok(StatusCode::NO_CONTENT);
    }

    s.data().auth_totp_set(auth.user.id, None, false).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate {
            changes: Changes::new().change("has_totp", &true, &false).build(),
        },
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth totp recovery exec
#[utoipa::path(
    post,
    path = "/auth/totp/recovery/exec",
    request_body = TotpVerificationRequest,
    tags = ["auth"],
    responses((status = OK, body = AuthState, description = "success")),
)]
async fn auth_totp_recovery_exec(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<TotpVerificationRequest>,
) -> Result<impl IntoResponse> {
    let (secret, enabled) = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .ok_or(Error::BadStatic("totp not enabled"))?;

    if !enabled {
        return Err(Error::BadStatic("totp not enabled"));
    }

    s.data()
        .auth_totp_recovery_use(auth.user.id, &json.code)
        .await?;

    s.data()
        .session_set_status(
            auth.session.id,
            SessionStatus::Sudo {
                user_id: auth.user.id,
                sudo_expires_at: Time::now_utc().saturating_add(Duration::minutes(5)).into(),
            },
        )
        .await?;
    s.services().sessions.invalidate(auth.session.id).await;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthSudo {
            session_id: auth.session.id,
        },
    })
    .await?;

    let auth_state = fetch_auth_state(&s, auth.user.id).await?;

    Ok(Json(auth_state))
}

/// Auth password set
#[utoipa::path(
    put,
    path = "/auth/password",
    tags = ["auth", "badge.sudo"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn auth_password_set(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PasswordSet>,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let data = s.data();
    let start_has_password = data.auth_password_get(auth.user.id).await?.is_some();

    let config = argon2::Config::default();
    let salt = {
        let mut salt = [0u8; 16];
        rand::fill(&mut salt);
        salt
    };
    let hash = argon2::hash_raw(json.password.as_bytes(), &salt, &config).unwrap();
    data.auth_password_set(auth.user.id, &hash, &salt).await?;

    let end_has_password = data.auth_password_get(auth.user.id).await?.is_some();

    let mut changes = Vec::new();
    changes.push(AuditLogChange {
        key: "has_password".into(),
        old: serde_json::to_value(start_has_password).unwrap(),
        new: serde_json::to_value(end_has_password).unwrap(),
    });

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let data = s.data();
    let has_password = data.auth_password_get(auth.user.id).await?.is_some();
    if !has_password {
        return Ok(StatusCode::NO_CONTENT);
    }

    data.auth_password_delete(auth.user.id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthUpdate {
            changes: Changes::new()
                .change("has_password", &has_password, &false)
                .build(),
        },
    })
    .await?;

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
    auth: Auth,
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
        data.session_set_status(auth.session.id, SessionStatus::Authorized { user_id })
            .await?;
        let srv = s.services();
        srv.sessions.invalidate(auth.session.id).await;
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: user_id.into_inner().into(),
            user_id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::SessionLogin {
                user_id,
                session_id: auth.session.id,
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
async fn auth_state(auth: Auth, State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let auth_state = fetch_auth_state(&s, auth.user.id).await?;
    Ok(Json(auth_state))
}

pub async fn fetch_auth_state(s: &ServerState, user_id: UserId) -> Result<AuthState> {
    let data = s.data();
    let oauth_providers = data.auth_oauth_get_all(user_id).await?;
    let email = data.user_email_list(user_id).await?;
    let password = data.auth_password_get(user_id).await?;
    let totp = data.auth_totp_get(user_id).await?;
    let auth_state = AuthState {
        has_email: email.iter().any(|e| e.is_verified && e.is_primary),
        has_totp: totp.map(|(_, enabled)| enabled).unwrap_or(false),
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
async fn auth_captcha_init(_auth: Auth, State(_s): State<Arc<ServerState>>) -> Result<Json<()>> {
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
    _auth: Auth,
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
    _auth: Auth,
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
    _auth: Auth,
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
    _auth: Auth,
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
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_authenticator_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Auth sudo upgrade (TEMP)
///
/// instantly upgrade to sudo mode; this is intended for debugging
#[utoipa::path(
    post,
    path = "/auth/_sudo",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "ok")),
)]
async fn auth_sudo_upgrade(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    s.data()
        .session_set_status(
            auth.session.id,
            SessionStatus::Sudo {
                user_id: auth.user.id,
                sudo_expires_at: Time::now_utc().saturating_add(Duration::minutes(5)).into(),
            },
        )
        .await?;
    s.services().sessions.invalidate(auth.session.id).await;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::AuthSudo {
            session_id: auth.session.id,
        },
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth sudo delete
///
/// downgrade yourself from sudo mode
#[utoipa::path(
    post,
    path = "/auth/sudo",
    tags = ["auth"],
    responses((status = NO_CONTENT, description = "ok")),
)]
async fn auth_sudo_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    s.data()
        .session_set_status(
            auth.session.id,
            SessionStatus::Authorized {
                user_id: auth.user.id,
            },
        )
        .await?;
    s.services().sessions.invalidate(auth.session.id).await;
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
        .routes(routes!(auth_totp_complete))
        .routes(routes!(auth_totp_exec))
        .routes(routes!(auth_totp_delete))
        .routes(routes!(auth_totp_recovery_get))
        .routes(routes!(auth_totp_recovery_rotate))
        .routes(routes!(auth_totp_recovery_exec))
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
        .routes(routes!(auth_sudo_upgrade))
        .routes(routes!(auth_sudo_delete))
}
