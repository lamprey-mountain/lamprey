use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::email::{EmailAddr, EmailInfo};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::UserId;
use common::v1::types::{AuditLogEntryType, MessageSync};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::types::UserIdReq;
use crate::{routes2, Error, ServerState};

use super::auth::fetch_auth_state;

/// Check if a user can still login after potentially removing an auth method
/// This prevents users from removing all auth methods which would lock them out
async fn ensure_can_still_login_after_email_removal(
    s: &ServerState,
    user_id: UserId,
) -> Result<()> {
    let mut auth_state = fetch_auth_state(s, user_id).await?;

    auth_state.has_email = false;

    if !auth_state.has_email
        && auth_state.oauth_providers.is_empty()
        && auth_state.authenticators.is_empty()
    {
        if auth_state.has_password {
            return Err(ApiError::from_code(ErrorCode::CannotRemoveLastAuthMethod).into());
        }

        return Err(ApiError::from_code(ErrorCode::CannotRemoveLastAuthMethod).into());
    }

    Ok(())
}

/// Email add
#[handler(routes::email_add)]
async fn email_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::email_add::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let email_addr: EmailAddr = req.addr.try_into()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let email_info = EmailInfo {
        email: email_addr.clone(),
        is_verified: false,
        is_primary: false,
    };

    match s
        .data()
        .user_email_add(target_user_id, email_info, s.config.max_user_emails)
        .await
    {
        Ok(_) => {
            let code = s
                .data()
                .user_email_verify_create(target_user_id, email_addr.clone())
                .await?;

            let query = url::form_urlencoded::Serializer::new(String::new())
                .append_pair("email", email_addr.as_ref())
                .append_pair("code", &code)
                .finish();
            let verification_link = format!("{}/verify-email?{}", s.config.html_url, query);

            s.services
                .email
                .send(
                    email_addr.clone(),
                    "Verify your email address".to_string(),
                    format!(
                        "Your verification code is: {}. You can also click this link: {}",
                        code, verification_link
                    ),
                    Some(format!(
                        "Your verification code is: <strong>{}</strong>. <br> You can also click this link: <a href=\"{}\">{}</a>",
                        code, &verification_link, &verification_link
                    )),
                )
                .await?;

            let al = auth.audit_log(target_user_id.into_inner().into());
            al.commit_success(AuditLogEntryType::EmailCreate {
                email: email_addr,
                changes: Changes::new()
                    .add("is_verified", &false)
                    .add("is_primary", &false)
                    .build(),
            })
            .await?;

            let user = s
                .services()
                .users
                .get(target_user_id, Some(auth.user.id))
                .await?;
            s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;

            Ok(StatusCode::CREATED.into_response())
        }
        Err(Error::EmailAlreadyExists) => Ok(StatusCode::OK.into_response()),
        Err(e) => Err(e),
    }
}

/// Email delete
#[handler(routes::email_delete)]
async fn email_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::email_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let email: EmailAddr = req.addr.try_into()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let emails = s.data().user_email_list(target_user_id).await?;
    let email_info = emails
        .iter()
        .find(|e| e.email == email)
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownUserEmail,
        )))?;

    if email_info.is_verified && email_info.is_primary {
        ensure_can_still_login_after_email_removal(&s, target_user_id).await?;
    }

    s.data()
        .user_email_delete(target_user_id, email.clone())
        .await?;

    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::EmailDelete {
        email,
        changes: Changes::new()
            .remove("is_verified", &email_info.is_verified)
            .build(),
    })
    .await?;

    let user = s
        .services()
        .users
        .get(target_user_id, Some(auth.user.id))
        .await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Email list
#[handler(routes::email_list)]
async fn email_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::email_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let emails = s.data().user_email_list(target_user_id).await?;

    Ok(Json(emails).into_response())
}

/// Email update
#[handler(routes::email_update)]
async fn email_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::email_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let email_addr: EmailAddr = req.addr.try_into()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let emails = s.data().user_email_list(target_user_id).await?;
    let email_info = emails
        .iter()
        .find(|e| e.email == email_addr)
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownUserEmail,
        )))?;

    if req.patch.is_primary == Some(true) {
        if !email_info.is_verified {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }
    }

    s.data()
        .user_email_update(target_user_id, email_addr.clone(), req.patch)
        .await?;

    let emails_new = s.data().user_email_list(target_user_id).await?;
    let email_info_new =
        emails_new
            .iter()
            .find(|e| e.email == email_addr)
            .ok_or(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownUserEmail,
            )))?;

    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::EmailUpdate {
        email: email_addr,
        changes: Changes::new()
            .change(
                "is_verified",
                &email_info.is_primary,
                &email_info_new.is_primary,
            )
            .build(),
    })
    .await?;

    let user = s
        .services()
        .users
        .get(target_user_id, Some(auth.user.id))
        .await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Email verification resend
#[handler(routes::email_verification_resend)]
async fn email_verification_resend(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::email_verification_resend::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let email_addr: EmailAddr = req.addr.try_into()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let emails = s.data().user_email_list(target_user_id).await?;
    let email_info = emails
        .into_iter()
        .find(|e| e.email == email_addr)
        .ok_or(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownUserEmail,
        )))?;

    if email_info.is_verified {
        return Ok(StatusCode::NO_CONTENT.into_response());
    }

    let code = s
        .data()
        .user_email_verify_create(target_user_id, email_addr.clone())
        .await?;

    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("email", email_addr.as_ref())
        .append_pair("code", &code)
        .finish();
    let verification_link = format!("{}/verify-email?{}", s.config.html_url, query);

    s.services
        .email
        .send(
            email_addr,
            "Verify your email address".to_string(),
            format!(
                "Your verification code is: {}. You can also click this link: {}",
                code, verification_link
            ),
            Some(format!(
                "Your verification code is: <strong>{}</strong>. <br> You can also click this link: <a href=\"{}\">{}</a>",
                code, &verification_link, &verification_link
            )),
        )
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Email verify finish
#[handler(routes::email_verification_finish)]
async fn email_verification_finish(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::email_verification_finish::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let email_addr: EmailAddr = req.addr.try_into()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    s.data()
        .user_email_verify_use(target_user_id, email_addr.clone(), req.code)
        .await?;

    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::EmailUpdate {
        email: email_addr,
        changes: Changes::new().change("is_verified", &false, &true).build(),
    })
    .await?;

    let user = s
        .services()
        .users
        .get(target_user_id, Some(auth.user.id))
        .await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(email_add))
        .routes(routes2!(email_list))
        .routes(routes2!(email_delete))
        .routes(routes2!(email_update))
        .routes(routes2!(email_verification_resend))
        .routes(routes2!(email_verification_finish))
}
