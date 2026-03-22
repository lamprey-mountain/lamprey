use std::sync::Arc;

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use common::v1::routes;
use common::v1::types::auth::{
    AuthState, CaptchaChallenge, PasswordExecIdent, TotpInit, TotpRecoveryCode, TotpRecoveryCodes,
};
use common::v1::types::email::EmailAddr;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus, AuditLogEntryType, MessageSync,
    RoomMemberPut, SessionStatus, UserId, SERVER_ROOM_ID,
};
use http::StatusCode;
use lamprey_macros::handler;
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Secret as TotpSecret, TOTP as Totp};
use tracing::debug;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::routes::util::{Auth, AuthRelaxed2};
use crate::types::DbUserCreate;
use crate::types::EmailPurpose;
use crate::{routes2, ServerState};

/// Auth oauth init
#[handler(routes::auth_oauth_init)]
async fn auth_oauth_init(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_oauth_init::Request,
) -> Result<impl IntoResponse> {
    let url = s
        .services()
        .oauth
        .create_url(&req.provider, auth.session.id)?;
    Ok(Json(routes::OauthInitResponse { url }))
}

/// Auth oauth redirect
#[handler(routes::auth_oauth_redirect)]
async fn auth_oauth_redirect(
    State(s): State<Arc<ServerState>>,
    req: routes::auth_oauth_redirect::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let provider_config = s
        .config
        .oauth_provider
        .get(&req.provider)
        .ok_or(Error::Unimplemented)?;

    let state =
        Uuid::parse_str(&req.state).map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;

    match req.provider.as_str() {
        "discord" => {
            let (oauth_token, session_id) =
                srv.oauth.exchange_code_for_token(state, req.code).await?;
            let u = srv.oauth.discord_get_user(oauth_token.access_token).await?;
            debug!("new discord user {:?}", u);
            let user_id = match data
                .auth_oauth_get_remote("discord".into(), u.user.id.clone())
                .await
            {
                Ok(user_id) => {
                    let user = srv.users.get(user_id, None).await?;
                    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                        .await?;
                    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

                    if provider_config.autoregister && user.registered_at.is_none() {
                        data.user_set_registered(user.id, Some(Time::now_utc()), None)
                            .await?;
                        srv.users.invalidate(user.id).await;
                        let updated_user = srv.users.get(user.id, None).await?;
                        let session = srv.sessions.get(session_id).await?;
                        let entry = AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: None,
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                            status: AuditLogEntryStatus::Success,
                            started_at: Time::now_utc(),
                            ended_at: Time::now_utc(),
                            ip_addr: session.ip_addr,
                            user_agent: session.user_agent,
                            application_id: session.app_id,
                        };
                        data.audit_logs_room_append(entry.clone()).await?;
                        s.broadcast_room(
                            entry.room_id,
                            entry.user_id,
                            MessageSync::AuditLogEntryCreate { entry },
                        )
                        .await?;
                        s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
                    }
                    user_id
                }
                Err(Error::ApiError(ApiError {
                    code: ErrorCode::UnknownUser,
                    ..
                })) => {
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
                    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                        .await?;
                    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

                    if provider_config.autoregister {
                        let session = srv.sessions.get(session_id).await?;
                        let entry = AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: None,
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                            status: AuditLogEntryStatus::Success,
                            started_at: Time::now_utc(),
                            ended_at: Time::now_utc(),
                            ip_addr: session.ip_addr,
                            user_agent: session.user_agent,
                            application_id: session.app_id,
                        };
                        data.audit_logs_room_append(entry.clone()).await?;
                        s.broadcast_room(
                            entry.room_id,
                            entry.user_id,
                            MessageSync::AuditLogEntryCreate { entry },
                        )
                        .await?;
                    }
                    data.auth_oauth_put("discord".into(), user.id, u.user.id.clone(), true)
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
            let entry = AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: (*user_id).into(),
                user_id,
                session_id: Some(session_id),
                reason: None,
                ty: AuditLogEntryType::SessionLogin {
                    user_id,
                    session_id,
                },
                status: AuditLogEntryStatus::Success,
                started_at: session.authorized_at.unwrap_or_else(Time::now_utc),
                ended_at: Time::now_utc(),
                ip_addr: session.ip_addr.clone(),
                user_agent: session.user_agent.clone(),
                application_id: session.app_id,
            };
            data.audit_logs_room_append(entry.clone()).await?;
            s.broadcast_room(
                entry.room_id,
                entry.user_id,
                MessageSync::AuditLogEntryCreate { entry },
            )
            .await?;
            Ok(Html(include_str!("../oauth.html")))
        }
        "github" => {
            let (oauth_token, session_id) =
                srv.oauth.exchange_code_for_token(state, req.code).await?;
            let u = srv.oauth.github_get_user(oauth_token.access_token).await?;
            debug!("new github user {:?}", u);
            let user_id = match data
                .auth_oauth_get_remote("github".into(), u.id.to_string())
                .await
            {
                Ok(user_id) => {
                    let user = srv.users.get(user_id, None).await?;
                    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                        .await?;
                    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

                    if provider_config.autoregister && user.registered_at.is_none() {
                        data.user_set_registered(user.id, Some(Time::now_utc()), None)
                            .await?;
                        srv.users.invalidate(user.id).await;
                        let updated_user = srv.users.get(user.id, None).await?;
                        let session = srv.sessions.get(session_id).await?;
                        let entry = AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: None,
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                            status: AuditLogEntryStatus::Success,
                            started_at: Time::now_utc(),
                            ended_at: Time::now_utc(),
                            ip_addr: session.ip_addr,
                            user_agent: session.user_agent,
                            application_id: session.app_id,
                        };
                        data.audit_logs_room_append(entry.clone()).await?;
                        s.broadcast_room(
                            entry.room_id,
                            entry.user_id,
                            MessageSync::AuditLogEntryCreate { entry },
                        )
                        .await?;
                        s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
                    }
                    user_id
                }
                Err(Error::ApiError(ApiError {
                    code: ErrorCode::UnknownUser,
                    ..
                })) => {
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
                    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                        .await?;
                    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

                    if provider_config.autoregister {
                        let session = srv.sessions.get(session_id).await?;
                        let entry = AuditLogEntry {
                            id: AuditLogEntryId::new(),
                            room_id: SERVER_ROOM_ID,
                            user_id: user.id,
                            session_id: Some(session_id),
                            reason: None,
                            ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                            status: AuditLogEntryStatus::Success,
                            started_at: Time::now_utc(),
                            ended_at: Time::now_utc(),
                            ip_addr: session.ip_addr,
                            user_agent: session.user_agent,
                            application_id: session.app_id,
                        };
                        data.audit_logs_room_append(entry.clone()).await?;
                        s.broadcast_room(
                            entry.room_id,
                            entry.user_id,
                            MessageSync::AuditLogEntryCreate { entry },
                        )
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
            let entry = AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: (*user_id).into(),
                user_id,
                session_id: Some(session_id),
                reason: None,
                ty: AuditLogEntryType::SessionLogin {
                    user_id,
                    session_id,
                },
                status: AuditLogEntryStatus::Success,
                started_at: session.authorized_at.unwrap_or_else(Time::now_utc),
                ended_at: Time::now_utc(),
                ip_addr: session.ip_addr.clone(),
                user_agent: session.user_agent.clone(),
                application_id: session.app_id,
            };
            data.audit_logs_room_append(entry.clone()).await?;
            s.broadcast_room(
                entry.room_id,
                entry.user_id,
                MessageSync::AuditLogEntryCreate { entry },
            )
            .await?;
            Ok(Html(include_str!("../oauth.html")))
        }
        _ => Err(Error::Unimplemented),
    }
}

/// Auth totp init
#[handler(routes::auth_totp_init)]
async fn auth_totp_init(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::auth_totp_init::Request,
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

/// Auth totp enable
#[handler(routes::auth_totp_enable)]
async fn auth_totp_enable(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_totp_enable::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let (secret, enabled) = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotInitialized))?;

    if enabled {
        return Err(ApiError::from_code(ErrorCode::TotpAlreadyEnabled).into());
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

    if !totp.check_current(&req.verification.code).unwrap_or(false) {
        return Err(ApiError::from_code(ErrorCode::InvalidTotpCode).into());
    }

    s.data()
        .auth_totp_set(auth.user.id, Some(secret), true)
        .await?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new().change("has_totp", &false, &true).build(),
    })
    .await?;

    let auth_state = fetch_auth_state(&s, auth.user.id).await?;

    Ok(Json(auth_state))
}

/// Auth totp recovery codes get
#[handler(routes::auth_totp_recovery_codes_get)]
async fn auth_totp_recovery_codes_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::auth_totp_recovery_codes_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let data = s.data();

    // Get existing recovery codes without generating new ones
    let existing_codes: Vec<(String, Option<Time>)> =
        data.auth_totp_recovery_get_all(auth.user.id).await?;

    let codes: Vec<TotpRecoveryCode> = existing_codes
        .into_iter()
        .map(|(code_str, used_at)| TotpRecoveryCode {
            code: code_str,
            used_at,
        })
        .collect();

    Ok(Json(TotpRecoveryCodes { codes }))
}

/// Auth totp recovery codes rotate
#[handler(routes::auth_totp_recovery_codes_rotate)]
async fn auth_totp_recovery_codes_rotate(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::auth_totp_recovery_codes_rotate::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    let codes: Vec<TotpRecoveryCode> = (0..5)
        .map(|_| TotpRecoveryCode {
            code: Uuid::new_v4().to_string(),
            used_at: None,
        })
        .collect();

    let code_strings: Vec<String> = codes.iter().map(|c| c.code.clone()).collect();
    s.data()
        .auth_totp_recovery_generate(auth.user.id, &code_strings)
        .await?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new()
            .change("totp_recovery_codes_rotated", &true, &true)
            .build(),
    })
    .await?;

    Ok(Json(TotpRecoveryCodes { codes }))
}

/// Auth password set
#[handler(routes::auth_password_set)]
async fn auth_password_set(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_password_set::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;

    let salt: [u8; 32] = rand::random();
    let config = argon2::Config::default();
    let hash = argon2::hash_raw(req.password.password.as_bytes(), &salt, &config).map_err(|e| {
        tracing::error!("failed to hash password: {}", e);
        Error::Internal("failed to hash password".to_owned())
    })?;

    s.data()
        .auth_password_set(auth.user.id, &hash, &salt)
        .await?;

    let start_state = fetch_auth_state(&s, auth.user.id).await?;
    let end_state = fetch_auth_state(&s, auth.user.id).await?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new()
            .change(
                "has_password",
                &start_state.has_password,
                &end_state.has_password,
            )
            .build(),
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Auth password exec
#[handler(routes::auth_password_exec)]
async fn auth_password_exec(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_password_exec::Request,
) -> Result<impl IntoResponse> {
    if auth.session.status != SessionStatus::Unauthorized {
        return Err(ApiError::from_code(ErrorCode::AlreadyAuthenticated).into());
    }

    let user_id = match &req.password.ident {
        PasswordExecIdent::UserId { user_id } => *user_id,
        PasswordExecIdent::Email { email } => s.data().user_email_lookup(email).await?,
    };

    let Some((stored_hash, stored_salt)) = s.data().auth_password_get(user_id).await? else {
        return Err(ApiError::from_code(ErrorCode::InvalidPassword).into());
    };

    let config = argon2::Config::default();
    let valid = argon2::verify_raw(
        &stored_hash,
        &stored_salt,
        req.password.password.as_bytes(),
        &config,
    )
    .unwrap_or(false);

    if !valid {
        return Err(ApiError::from_code(ErrorCode::InvalidPassword).into());
    }

    let user = s.services().users.get(user_id, None).await?;
    let session = auth.session.clone();

    let (_totp_secret, totp_enabled) = s
        .data()
        .auth_totp_get(user_id)
        .await?
        .unwrap_or((String::new(), false));

    let status = if totp_enabled {
        SessionStatus::Unauthorized
    } else {
        SessionStatus::Authorized { user_id }
    };

    s.data().session_set_status(session.id, status).await?;
    s.services().sessions.invalidate(session.id).await;

    if !totp_enabled {
        let al = Auth {
            user: user.clone(),
            real_user: None,
            session: session.clone(),
            scopes: auth.scopes.clone(),
            reason: auth.reason.clone(),
            audit_log_slot: auth.audit_log_slot.clone(),
            s: auth.s.clone(),
        }
        .audit_log(user_id.into_inner().into());
        al.commit_success(AuditLogEntryType::SessionLogin {
            user_id,
            session_id: session.id,
        })
        .await?;
    }

    let auth_state = fetch_auth_state(&s, user_id).await?;
    Ok(Json(auth_state))
}

/// Auth captcha challenge
#[handler(routes::auth_captcha_challenge)]
async fn auth_captcha_challenge(
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_captcha_challenge::Request,
) -> Result<impl IntoResponse> {
    // TODO: implement captcha
    Ok(Json(CaptchaChallenge {
        code: String::new(),
    }))
}

/// Auth captcha init
#[handler(routes::auth_captcha_init)]
async fn auth_captcha_init(
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_captcha_init::Request,
) -> Result<impl IntoResponse> {
    // TODO: implement captcha
    Ok(StatusCode::NO_CONTENT)
}

/// Auth captcha submit
#[handler(routes::auth_captcha_submit)]
async fn auth_captcha_submit(
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_captcha_submit::Request,
) -> Result<impl IntoResponse> {
    // TODO: implement captcha
    Ok(StatusCode::NO_CONTENT)
}

/// Auth webauthn init
#[handler(routes::auth_webauthn_init)]
async fn auth_webauthn_init(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_init::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(StatusCode::NO_CONTENT)
}

/// Auth webauthn exec
#[handler(routes::auth_webauthn_exec)]
async fn auth_webauthn_exec(
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_exec::Request,
) -> Result<impl IntoResponse> {
    // TODO: implement WebAuthn
    Ok(StatusCode::NO_CONTENT)
}

/// Auth webauthn patch
#[handler(routes::auth_webauthn_patch)]
async fn auth_webauthn_patch(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_patch::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(StatusCode::NO_CONTENT)
}

/// Auth webauthn delete
#[handler(routes::auth_webauthn_delete)]
async fn auth_webauthn_delete(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(StatusCode::NO_CONTENT)
}

/// Auth sudo upgrade
#[handler(routes::auth_sudo_upgrade)]
async fn auth_sudo_upgrade(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_sudo_upgrade::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement sudo upgrade
    Ok(StatusCode::NO_CONTENT)
}

/// Auth sudo delete
#[handler(routes::auth_sudo_delete)]
async fn auth_sudo_delete(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_sudo_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement sudo delete
    Ok(StatusCode::NO_CONTENT)
}

/// Auth oauth delete
///
/// Remove an oauth provider
#[handler(routes::auth_oauth_delete)]
async fn auth_oauth_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_oauth_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;

    ensure_can_still_login_after_removal(&s, auth.user.id, "oauth", Some(&req.provider)).await?;

    let start_state = fetch_auth_state(&s, auth.user.id).await?;
    let data = s.data();
    data.auth_oauth_delete(req.provider, auth.user.id).await?;
    let end_state = fetch_auth_state(&s, auth.user.id).await?;
    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new()
            .change(
                "oauth_providers",
                &start_state.oauth_providers,
                &end_state.oauth_providers,
            )
            .build(),
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth email exec
///
/// Send a "magic link" email to login
#[handler(routes::auth_email_exec)]
async fn auth_email_exec(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_email_exec::Request,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let code = Uuid::new_v4().to_string();
    let email: EmailAddr = req
        .addr
        .try_into()
        .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
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
#[handler(routes::auth_email_reset)]
async fn auth_email_reset(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_email_reset::Request,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let code = Uuid::new_v4().to_string();
    let email: EmailAddr = req
        .addr
        .try_into()
        .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
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
#[handler(routes::auth_email_complete)]
async fn auth_email_complete(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_email_complete::Request,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let email: EmailAddr = req
        .addr
        .try_into()
        .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
    let (req_addr, req_session, purpose) = d.auth_email_use(req.complete.code).await?;

    if req_addr != email {
        debug!("wrong email");
        return Err(ApiError::from_code(ErrorCode::InvalidOrExpiredCode).into());
    }

    if req_session != auth.session.id {
        debug!("wrong session");
        return Err(ApiError::from_code(ErrorCode::InvalidOrExpiredCode).into());
    }

    if auth.session.status != SessionStatus::Unauthorized {
        debug!("already authenticated");
        return Err(ApiError::from_code(ErrorCode::InvalidOrExpiredCode).into());
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
            let srv = s.services();
            let user = srv.users.get(user_id, None).await?;
            let al = Auth {
                user: user.clone(),
                real_user: None,
                session: session.clone(),
                scopes: auth.scopes.clone(),
                reason: auth.reason.clone(),
                audit_log_slot: auth.audit_log_slot.clone(),
                s: auth.s.clone(),
            }
            .audit_log(user_id.into_inner().into());
            al.commit_success(AuditLogEntryType::SessionLogin {
                user_id,
                session_id: session.id,
            })
            .await?;
        }
        EmailPurpose::Reset => {
            let srv = s.services();
            let user = srv.users.get(user_id, None).await?;
            let al = Auth {
                user: user.clone(),
                real_user: None,
                session: session.clone(),
                scopes: auth.scopes.clone(),
                reason: auth.reason.clone(),
                audit_log_slot: auth.audit_log_slot.clone(),
                s: auth.s.clone(),
            }
            .audit_log(user_id.into_inner().into());
            al.commit_success(AuditLogEntryType::AuthSudo {
                session_id: session.id,
            })
            .await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Auth totp exec
#[handler(routes::auth_totp_exec)]
async fn auth_totp_exec(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_totp_exec::Request,
) -> Result<impl IntoResponse> {
    let (secret, enabled) = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotEnabled))?;

    if !enabled {
        return Err(ApiError::from_code(ErrorCode::TotpNotEnabled).into());
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

    if !totp.check_current(&req.verification.code).unwrap_or(false) {
        return Err(ApiError::from_code(ErrorCode::InvalidTotpCode).into());
    }

    let expires_at = Time::now_utc().saturating_add(Duration::minutes(5));
    s.data()
        .session_set_status(
            auth.session.id,
            SessionStatus::Sudo {
                user_id: auth.user.id,
                sudo_expires_at: expires_at.into(),
            },
        )
        .await?;
    s.services().sessions.invalidate(auth.session.id).await;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthSudo {
        session_id: auth.session.id,
    })
    .await?;

    let auth_state = fetch_auth_state(&s, auth.user.id).await?;
    Ok(Json(auth_state))
}

/// Auth totp recovery exec
#[handler(routes::auth_totp_recovery_exec)]
async fn auth_totp_recovery_exec(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_totp_recovery_exec::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();

    let (_secret, enabled) = data
        .auth_totp_get(auth.user.id)
        .await?
        .ok_or_else(|| ApiError::from_code(ErrorCode::TotpNotEnabled))?;

    if !enabled {
        return Err(ApiError::from_code(ErrorCode::TotpNotEnabled).into());
    }

    // Try to use the recovery code
    data.auth_totp_recovery_use(auth.user.id, &req.verification.code)
        .await?;

    let expires_at = Time::now_utc().saturating_add(Duration::minutes(5));
    data.session_set_status(
        auth.session.id,
        SessionStatus::Sudo {
            user_id: auth.user.id,
            sudo_expires_at: expires_at.into(),
        },
    )
    .await?;
    s.services().sessions.invalidate(auth.session.id).await;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthSudo {
        session_id: auth.session.id,
    })
    .await?;

    let auth_state = fetch_auth_state(&s, auth.user.id).await?;
    Ok(Json(auth_state))
}

/// Auth totp delete
#[handler(routes::auth_totp_delete)]
async fn auth_totp_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::auth_totp_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;

    ensure_can_still_login_after_removal(&s, auth.user.id, "totp", None).await?;

    let had_totp = s
        .data()
        .auth_totp_get(auth.user.id)
        .await?
        .map(|(_, enabled)| enabled)
        .unwrap_or(false);

    if !had_totp {
        return Ok(StatusCode::NO_CONTENT);
    }

    if s.data().user_owns_room_requiring_mfa(auth.user.id).await? {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }

    s.data().auth_totp_set(auth.user.id, None, false).await?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new().change("has_totp", &true, &false).build(),
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Auth password delete
#[handler(routes::auth_password_delete)]
async fn auth_password_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::auth_password_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;

    let has_password = s.data().auth_password_get(auth.user.id).await?.is_some();

    if !has_password {
        return Ok(StatusCode::NO_CONTENT);
    }

    ensure_can_still_login_after_removal(&s, auth.user.id, "password", None).await?;

    let start_state = fetch_auth_state(&s, auth.user.id).await?;
    let data = s.data();
    data.auth_password_delete(auth.user.id).await?;
    let end_state = fetch_auth_state(&s, auth.user.id).await?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new()
            .change(
                "has_password",
                &start_state.has_password,
                &end_state.has_password,
            )
            .build(),
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Auth state
///
/// Get the available auth methods for this user
#[handler(routes::auth_state)]
async fn auth_state(
    auth: AuthRelaxed2,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_state::Request,
) -> Result<impl IntoResponse> {
    let s = auth.s.clone();
    let user_id = auth
        .session
        .user_id()
        .ok_or_else(|| Error::BadStatic("unknown user for session"))?;
    let state = fetch_auth_state(&s, user_id).await?;
    Ok(Json(state))
}

// Helper function to check if user can still login after removing an auth method
async fn ensure_can_still_login_after_removal(
    s: &ServerState,
    user_id: UserId,
    method: &str,
    provider: Option<&str>,
) -> Result<()> {
    let mut auth_state = fetch_auth_state(s, user_id).await?;

    // Temporarily "remove" the auth method to simulate the state after removal
    match method {
        "oauth" => {
            if let Some(provider) = provider {
                auth_state.oauth_providers.retain(|p| p != provider);
            }
        }
        "password" => auth_state.has_password = false,
        _ => {}
    }

    // Check if the user can still login with remaining methods
    if !auth_state.can_login() {
        return Err(ApiError::from_code(ErrorCode::CannotRemoveLastAuthMethod).into());
    }

    Ok(())
}

// Helper function - used by other routes
pub async fn fetch_auth_state(s: &ServerState, user_id: UserId) -> Result<AuthState> {
    let data = s.data();

    let (_totp_secret, totp_enabled) = data
        .auth_totp_get(user_id)
        .await?
        .unwrap_or((String::new(), false));
    let password_exists = data.auth_password_get(user_id).await?.is_some();
    let oauth_providers = data.auth_oauth_get_all(user_id).await?;
    let has_email = !data.user_email_list(user_id).await?.is_empty();
    let authenticators = vec![]; // TODO: implement webauthn

    Ok(AuthState {
        has_email,
        has_totp: totp_enabled,
        has_password: password_exists,
        oauth_providers,
        authenticators,
    })
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(auth_oauth_init))
        .routes(routes2!(auth_oauth_redirect))
        .routes(routes2!(auth_oauth_delete))
        .routes(routes2!(auth_email_exec))
        .routes(routes2!(auth_email_reset))
        .routes(routes2!(auth_email_complete))
        .routes(routes2!(auth_totp_init))
        .routes(routes2!(auth_totp_enable))
        .routes(routes2!(auth_totp_delete))
        .routes(routes2!(auth_totp_exec))
        .routes(routes2!(auth_totp_recovery_codes_get))
        .routes(routes2!(auth_totp_recovery_codes_rotate))
        .routes(routes2!(auth_totp_recovery_exec))
        .routes(routes2!(auth_password_set))
        .routes(routes2!(auth_password_delete))
        .routes(routes2!(auth_password_exec))
        .routes(routes2!(auth_state))
        .routes(routes2!(auth_webauthn_init))
        .routes(routes2!(auth_webauthn_exec))
        .routes(routes2!(auth_webauthn_patch))
        .routes(routes2!(auth_webauthn_delete))
        .routes(routes2!(auth_captcha_challenge))
        .routes(routes2!(auth_captcha_init))
        .routes(routes2!(auth_captcha_submit))
        .routes(routes2!(auth_sudo_upgrade))
        .routes(routes2!(auth_sudo_delete))
}
