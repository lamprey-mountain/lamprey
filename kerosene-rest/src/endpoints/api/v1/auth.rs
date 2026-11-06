use common::v1::types::auth::{CaptchaChallenge, TotpInit, TotpRecoveryCodes};
use common::v1::types::util::Changes;
use common::v1::types::{AuditLogEntryType, Session, SessionStatus, UserId};
use lamprey_backend_services::compat::types::EmailPurpose;

use crate::prelude::*;

#[handler(routes::auth_oauth_init)]
pub async fn oauth_init(
    req: Req<routes::auth_oauth_init::Endpoint>,
) -> Result<routes::auth_oauth_init::Response> {
    let session = req.identity().ensure_session()?;
    let url = req
        .services()
        .oauth
        .create_url(&req.inner().provider, session.id)
        .cast_internal()?;
    Ok(routes::auth_oauth_init::Response {
        oauth: routes::OauthInitResponse { url },
    })
}

#[handler(routes::auth_state)]
pub async fn state(req: Req<routes::auth_state::Endpoint>) -> Result<routes::auth_state::Response> {
    let session = req.identity().ensure_session()?;
    let user_id = session.user_id().ok_or_else(|| {
        ApiError::with_message(
            ErrorCode::MissingAuth,
            "unknown user for session".to_owned(),
        )
    })?;

    let state = req
        .services()
        .auth
        .get_state(user_id)
        .await
        .cast_internal()?;
    Ok(routes::auth_state::Response { state })
}

#[handler(routes::auth_totp_init)]
pub async fn totp_init(
    req: Req<routes::auth_totp_init::Endpoint>,
) -> Result<routes::auth_totp_init::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    let secret = req
        .services()
        .auth
        .init_totp(user.id)
        .await
        .cast_internal()?;

    Ok(routes::auth_totp_init::Response {
        totp: TotpInit { secret },
    })
}

#[handler(routes::auth_totp_enable)]
pub async fn totp_enable(
    req: Req<routes::auth_totp_enable::Endpoint>,
) -> Result<routes::auth_totp_enable::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    let changes = req
        .services()
        .auth
        .enable_totp(user.id, &req.inner().verification.code)
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::AuthUpdate { changes },
        )
        .success();

    Ok(routes::auth_totp_enable::Response {})
}

#[handler(routes::auth_totp_delete)]
pub async fn totp_delete(
    req: Req<routes::auth_totp_delete::Endpoint>,
) -> Result<routes::auth_totp_delete::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;
    let srv = req.services();

    if let Some(changes) = srv.auth.delete_totp(user.id).await.cast_internal()? {
        req.audit_log()
            .push(
                user.id.into_inner().into(),
                AuditLogEntryType::AuthUpdate { changes },
            )
            .success();
    }

    let state = srv.auth.get_state(user.id).await.cast_internal()?;
    Ok(routes::auth_totp_delete::Response { state })
}

/// get which user a session is bound to, rejecting expired sessions
fn mfa_user_id(session: &Session) -> Result<UserId> {
    let user_id = session.user_id().ok_or_else(|| {
        ApiError::with_message(ErrorCode::MissingAuth, "session requires a user".to_owned())
    })?;

    if session.status.bound_expired() {
        return Err(
            ApiError::with_message(ErrorCode::MissingAuth, "session expired".to_owned()).into(),
        );
    }

    Ok(user_id)
}

#[handler(routes::auth_totp_exec)]
pub async fn totp_exec(
    req: Req<routes::auth_totp_exec::Endpoint>,
) -> Result<routes::auth_totp_exec::Response> {
    let session = req.identity().ensure_session()?;
    let session_id = session.id;
    let user_id = mfa_user_id(session)?;
    let was_unauthenticated = matches!(
        session.status,
        SessionStatus::Unauthorized | SessionStatus::Bound { .. }
    );

    let state = req
        .services()
        .auth
        .totp_exec(session_id, user_id, &req.inner().verification.code)
        .await
        .cast_internal()?;

    // SessionLogin must be recorded before AuthSudo when this endpoint authenticates the session
    let al = req.audit_log_as(user_id, session);
    if was_unauthenticated {
        al.push(
            user_id.into_inner().into(),
            AuditLogEntryType::SessionLogin {
                user_id,
                session_id,
            },
        )
        .success();
    }
    al.push(
        user_id.into_inner().into(),
        AuditLogEntryType::AuthSudo { session_id },
    )
    .success();

    Ok(routes::auth_totp_exec::Response { state })
}

#[handler(routes::auth_totp_recovery_codes_get)]
pub async fn recovery_codes_get(
    req: Req<routes::auth_totp_recovery_codes_get::Endpoint>,
) -> Result<routes::auth_totp_recovery_codes_get::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    let codes = req
        .services()
        .auth
        .recovery_codes_get(user.id)
        .await
        .cast_internal()?;

    Ok(routes::auth_totp_recovery_codes_get::Response {
        codes: TotpRecoveryCodes { codes },
    })
}

#[handler(routes::auth_totp_recovery_codes_rotate)]
pub async fn recovery_codes_rotate(
    req: Req<routes::auth_totp_recovery_codes_rotate::Endpoint>,
) -> Result<routes::auth_totp_recovery_codes_rotate::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    let codes = req
        .services()
        .auth
        .recovery_codes_rotate(user.id)
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::AuthUpdate {
                changes: Changes::new().build(),
            },
        )
        .success();

    Ok(routes::auth_totp_recovery_codes_rotate::Response {
        codes: TotpRecoveryCodes { codes },
    })
}

#[handler(routes::auth_totp_recovery_exec)]
pub async fn recovery_code_exec(
    req: Req<routes::auth_totp_recovery_exec::Endpoint>,
) -> Result<routes::auth_totp_recovery_exec::Response> {
    let session = req.identity().ensure_session()?;
    let session_id = session.id;
    let user_id = mfa_user_id(session)?;
    let was_unauthenticated = matches!(
        session.status,
        SessionStatus::Unauthorized | SessionStatus::Bound { .. }
    );

    let state = req
        .services()
        .auth
        .recovery_code_exec(session_id, user_id, &req.inner().verification.code)
        .await
        .cast_internal()?;

    // SessionLogin must be recorded before AuthSudo when this endpoint authenticates the session
    let al = req.audit_log_as(user_id, session);
    if was_unauthenticated {
        al.push(
            user_id.into_inner().into(),
            AuditLogEntryType::SessionLogin {
                user_id,
                session_id,
            },
        )
        .success();
    }
    al.push(
        user_id.into_inner().into(),
        AuditLogEntryType::AuthSudo { session_id },
    )
    .success();

    Ok(routes::auth_totp_recovery_exec::Response { state })
}

#[handler(routes::auth_password_set)]
pub async fn password_set(
    req: Req<routes::auth_password_set::Endpoint>,
) -> Result<routes::auth_password_set::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    let changes = req
        .services()
        .auth
        .set_password(user.id, &req.inner().password.password)
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::AuthUpdate { changes },
        )
        .success();

    Ok(routes::auth_password_set::Response {})
}

#[handler(routes::auth_password_delete)]
pub async fn password_delete(
    req: Req<routes::auth_password_delete::Endpoint>,
) -> Result<routes::auth_password_delete::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    if let Some(changes) = req
        .services()
        .auth
        .delete_password(user.id)
        .await
        .cast_internal()?
    {
        req.audit_log()
            .push(
                user.id.into_inner().into(),
                AuditLogEntryType::AuthUpdate { changes },
            )
            .success();
    }

    Ok(routes::auth_password_delete::Response {})
}

#[handler(routes::auth_password_exec)]
pub async fn password_exec(
    req: Req<routes::auth_password_exec::Endpoint>,
) -> Result<routes::auth_password_exec::Response> {
    let session = req.identity().ensure_session()?;
    if session.status != SessionStatus::Unauthorized && !session.status.bound_expired() {
        return Err(ApiError::from_code(ErrorCode::AlreadyAuthenticated).into());
    }

    let session_id = session.id;
    let srv = req.services();
    let inner = req.inner();

    let user_id = srv
        .auth
        .resolve_password_ident(&inner.password.ident)
        .await
        .cast_internal()?;
    let logged_in = srv
        .auth
        .password_exec(user_id, &inner.password.password, session_id)
        .await
        .cast_internal()?;

    if logged_in {
        // the request's identity is a guest, so the audit entry is pushed manually
        req.audit_log_as(user_id, session)
            .push(
                user_id.into_inner().into(),
                AuditLogEntryType::SessionLogin {
                    user_id,
                    session_id,
                },
            )
            .success();
    }

    Ok(routes::auth_password_exec::Response {})
}

#[handler(routes::auth_sudo_upgrade)]
pub async fn sudo_upgrade(
    req: Req<routes::auth_sudo_upgrade::Endpoint>,
) -> Result<routes::auth_sudo_upgrade::Response> {
    let session_id = req.identity().ensure_session()?.id;
    let user = req.identity().ensure_user()?;

    req.services()
        .auth
        .sudo_upgrade(session_id, user.id)
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::AuthSudo { session_id },
        )
        .success();

    Ok(routes::auth_sudo_upgrade::Response {})
}

#[handler(routes::auth_sudo_delete)]
pub async fn sudo_delete(
    req: Req<routes::auth_sudo_delete::Endpoint>,
) -> Result<routes::auth_sudo_delete::Response> {
    req.identity().ensure_sudo()?;
    let session_id = req.identity().ensure_session()?.id;
    let user = req.identity().ensure_user()?;

    req.services()
        .auth
        .sudo_downgrade(session_id, user.id)
        .await
        .cast_internal()?;

    Ok(routes::auth_sudo_delete::Response {})
}

#[handler(routes::auth_oauth_delete)]
pub async fn oauth_delete(
    req: Req<routes::auth_oauth_delete::Endpoint>,
) -> Result<routes::auth_oauth_delete::Response> {
    req.identity().ensure_sudo()?;
    let user = req.identity().ensure_user()?;

    let changes = req
        .services()
        .auth
        .delete_oauth(user.id, &req.inner().provider)
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::AuthUpdate { changes },
        )
        .success();

    Ok(routes::auth_oauth_delete::Response {})
}

#[handler(routes::auth_email_exec)]
pub async fn email_exec(
    req: Req<routes::auth_email_exec::Endpoint>,
) -> Result<routes::auth_email_exec::Response> {
    let session_id = req.identity().ensure_session()?.id;

    req.services()
        .auth
        .email_login(session_id, &req.inner().addr)
        .await
        .cast_internal()?;

    Ok(routes::auth_email_exec::Response {})
}

#[handler(routes::auth_email_reset)]
pub async fn email_reset(
    req: Req<routes::auth_email_reset::Endpoint>,
) -> Result<routes::auth_email_reset::Response> {
    let session_id = req.identity().ensure_session()?.id;

    req.services()
        .auth
        .email_reset(session_id, &req.inner().addr)
        .await
        .cast_internal()?;

    Ok(routes::auth_email_reset::Response {})
}

#[handler(routes::auth_email_complete)]
pub async fn email_complete(
    req: Req<routes::auth_email_complete::Endpoint>,
) -> Result<routes::auth_email_complete::Response> {
    let session = req.identity().ensure_session()?.clone();
    let inner = req.inner();

    let (user_id, purpose, mfa_pending) = req
        .services()
        .auth
        .email_complete(&session, &inner.addr, inner.complete.code.clone())
        .await
        .cast_internal()?;

    let al = req.audit_log_as(user_id, &session);
    match purpose {
        EmailPurpose::Authn => {
            // when a second factor is still pending, the follow-up totp_exec records the login
            if !mfa_pending {
                al.push(
                    user_id.into_inner().into(),
                    AuditLogEntryType::SessionLogin {
                        user_id,
                        session_id: session.id,
                    },
                )
                .success();
            }
        }
        EmailPurpose::Reset => {
            // a reset only enters sudo mode once the second factor is done
            if !mfa_pending {
                al.push(
                    user_id.into_inner().into(),
                    AuditLogEntryType::SessionLogin {
                        user_id,
                        session_id: session.id,
                    },
                )
                .success();
                al.push(
                    user_id.into_inner().into(),
                    AuditLogEntryType::AuthSudo {
                        session_id: session.id,
                    },
                )
                .success();
            }
        }
    }

    Ok(routes::auth_email_complete::Response {})
}

#[handler(routes::auth_captcha_challenge)]
pub async fn captcha_challenge(
    _req: Req<routes::auth_captcha_challenge::Endpoint>,
) -> Result<routes::auth_captcha_challenge::Response> {
    // TODO: implement captcha
    Ok(routes::auth_captcha_challenge::Response {
        captcha: CaptchaChallenge {
            code: String::new(),
        },
    })
}

#[handler(routes::auth_captcha_init)]
pub async fn captcha_init(
    _req: Req<routes::auth_captcha_init::Endpoint>,
) -> Result<routes::auth_captcha_init::Response> {
    // TODO: implement captcha
    Ok(routes::auth_captcha_init::Response {})
}

#[handler(routes::auth_captcha_submit)]
pub async fn captcha_submit(
    _req: Req<routes::auth_captcha_submit::Endpoint>,
) -> Result<routes::auth_captcha_submit::Response> {
    // TODO: implement captcha
    Ok(routes::auth_captcha_submit::Response {})
}

// TODO: implement WebAuthn
#[handler(routes::auth_webauthn_register_challenge)]
pub async fn webauthn_register_challenge(
    _req: Req<routes::auth_webauthn_register_challenge::Endpoint>,
) -> Result<routes::auth_webauthn_register_challenge::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::auth_webauthn_register_finish)]
pub async fn webauthn_register_finish(
    _req: Req<routes::auth_webauthn_register_finish::Endpoint>,
) -> Result<routes::auth_webauthn_register_finish::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::auth_webauthn_auth_challenge)]
pub async fn webauthn_auth_challenge(
    _req: Req<routes::auth_webauthn_auth_challenge::Endpoint>,
) -> Result<routes::auth_webauthn_auth_challenge::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::auth_webauthn_auth_finish)]
pub async fn webauthn_auth_finish(
    _req: Req<routes::auth_webauthn_auth_finish::Endpoint>,
) -> Result<routes::auth_webauthn_auth_finish::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::auth_webauthn_authenticator_list)]
pub async fn webauthn_authenticator_list(
    _req: Req<routes::auth_webauthn_authenticator_list::Endpoint>,
) -> Result<routes::auth_webauthn_authenticator_list::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::auth_webauthn_authenticator_patch)]
pub async fn webauthn_authenticator_patch(
    _req: Req<routes::auth_webauthn_authenticator_patch::Endpoint>,
) -> Result<routes::auth_webauthn_authenticator_patch::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::auth_webauthn_authenticator_delete)]
pub async fn webauthn_authenticator_delete(
    _req: Req<routes::auth_webauthn_authenticator_delete::Endpoint>,
) -> Result<routes::auth_webauthn_authenticator_delete::Response> {
    Err(Error::Unimplemented)
}

export_routes!(
    oauth_init,
    state,
    totp_init,
    totp_enable,
    totp_delete,
    totp_exec,
    recovery_codes_get,
    recovery_codes_rotate,
    recovery_code_exec,
    password_set,
    password_delete,
    password_exec,
    sudo_upgrade,
    sudo_delete,
    oauth_delete,
    email_exec,
    email_reset,
    email_complete,
    captcha_challenge,
    captcha_init,
    captcha_submit,
    webauthn_register_challenge,
    webauthn_register_finish,
    webauthn_auth_challenge,
    webauthn_auth_finish,
    webauthn_authenticator_list,
    webauthn_authenticator_patch,
    webauthn_authenticator_delete,
);
