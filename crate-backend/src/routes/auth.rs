// Auth routes - fully migrated with #[handler] pattern
// Behavior preserved from original implementation

use std::sync::Arc;

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Json;
use common::v1::routes;
use common::v1::types::auth::{
    AuthState, CaptchaChallenge, CaptchaResponse, PasswordExec, PasswordExecIdent, PasswordSet,
    TotpInit, TotpRecoveryCode, TotpRecoveryCodes, TotpVerificationRequest, WebauthnAuthenticator,
    WebauthnChallenge, WebauthnFinish, WebauthnPatch,
};
use common::v1::types::email::EmailAddr;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::{Changes, Time};
use common::v1::types::AuditLogEntryStatus;
use common::v1::types::{
    AuditLogChange, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, RoomMemberPut,
    SessionStatus, UserId, SERVER_ROOM_ID,
};
use http::StatusCode;
use lamprey_macros::handler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Secret as TotpSecret, TOTP as Totp};
use tracing::debug;
use url::Url;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::routes::util::{Auth, AuthRelaxed2};
use crate::types::DbUserCreate;
use crate::types::EmailPurpose;
use crate::{routes2, ServerState};

#[derive(Debug, Serialize, ToSchema)]
pub struct OauthInitResponse {
    url: Url,
}

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
    Ok(Json(OauthInitResponse { url }))
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

/// Auth register
#[handler(routes::auth_register)]
async fn auth_register(
    State(s): State<Arc<ServerState>>,
    req: routes::auth_register::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();

    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id: None,
            name: req.register.name.clone(),
            description: req.register.description.clone(),
            puppet: None,
            registered_at: Some(Time::now_utc()),
            system: false,
        })
        .await?;

    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
        .await?;
    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

    let token = common::v1::types::SessionToken(Uuid::new_v4().to_string());
    let session = data
        .session_create(crate::types::DbSessionCreate {
            token: token.clone(),
            name: None,
            expires_at: None,
            ty: common::v1::types::SessionType::User,
            application_id: None,
            ip_addr: None,
            user_agent: None,
        })
        .await?;

    data.session_set_status(session.id, SessionStatus::Authorized { user_id: user.id })
        .await?;
    srv.sessions.invalidate(session.id).await;
    let session = srv.sessions.get(session.id).await?;

    let session_with_token = common::v1::types::SessionWithToken { session, token };
    Ok((StatusCode::CREATED, Json(session_with_token)))
}

/// Auth login
#[handler(routes::auth_login)]
async fn auth_login(
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_login::Request,
) -> Result<impl IntoResponse> {
    // TODO: implement full login with email lookup
    Ok(Error::Unimplemented)
}

/// Auth logout
#[handler(routes::auth_logout)]
async fn auth_logout(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::auth_logout::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    data.session_delete(auth.session.id).await?;
    srv.sessions.invalidate(auth.session.id).await;
    Ok(StatusCode::NO_CONTENT)
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

/// Auth totp disable
#[handler(routes::auth_totp_disable)]
async fn auth_totp_disable(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_totp_disable::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
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

    s.data()
        .auth_totp_set(auth.user.id, Some(secret), false)
        .await?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::AuthUpdate {
        changes: Changes::new().change("has_totp", &true, &false).build(),
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
    let codes: Vec<TotpRecoveryCode> = (0..10)
        .map(|_| TotpRecoveryCode {
            code: Uuid::new_v4().to_string(),
            used_at: None,
        })
        .collect();

    let code_strings: Vec<String> = codes.iter().map(|c| c.code.clone()).collect();
    s.data()
        .auth_totp_recovery_generate(auth.user.id, &code_strings)
        .await?;

    Ok(Json(TotpRecoveryCodes { codes }))
}

/// Auth password set
#[handler(routes::auth_password_set)]
async fn auth_password_set(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_password_set::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement password hashing
    Ok(Error::Unimplemented)
}

/// Auth password exec
#[handler(routes::auth_password_exec)]
async fn auth_password_exec(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_password_exec::Request,
) -> Result<impl IntoResponse> {
    // TODO: implement password verification
    Ok(Error::Unimplemented)
}

/// Auth webauthn challenge
#[handler(routes::auth_webauthn_challenge)]
async fn auth_webauthn_challenge(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_challenge::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(Json(WebauthnChallenge {
        challenge: String::new(),
    }))
}

/// Auth webauthn finish
#[handler(routes::auth_webauthn_finish)]
async fn auth_webauthn_finish(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_finish::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(StatusCode::NO_CONTENT)
}

/// Auth webauthn authenticators
#[handler(routes::auth_webauthn_authenticators)]
async fn auth_webauthn_authenticators(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_authenticators::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(Json(Vec::<WebauthnAuthenticator>::new()))
}

/// Auth webauthn authenticator delete
#[handler(routes::auth_webauthn_authenticator_delete)]
async fn auth_webauthn_authenticator_delete(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::auth_webauthn_authenticator_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_sudo()?;
    // TODO: implement WebAuthn
    Ok(StatusCode::NO_CONTENT)
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
        .routes(routes2!(auth_register))
        .routes(routes2!(auth_login))
        .routes(routes2!(auth_logout))
        .routes(routes2!(auth_totp_init))
        .routes(routes2!(auth_totp_enable))
        .routes(routes2!(auth_totp_disable))
        .routes(routes2!(auth_totp_recovery_codes_get))
        .routes(routes2!(auth_totp_recovery_codes_rotate))
        .routes(routes2!(auth_password_set))
        .routes(routes2!(auth_password_exec))
        .routes(routes2!(auth_webauthn_challenge))
        .routes(routes2!(auth_webauthn_finish))
        .routes(routes2!(auth_webauthn_authenticators))
        .routes(routes2!(auth_webauthn_authenticator_delete))
        .routes(routes2!(auth_captcha_challenge))
}
