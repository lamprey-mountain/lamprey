use std::sync::Arc;

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::types::email::{EmailAddr, EmailInfo, EmailInfoPatch};
use common::v1::types::util::Changes;
use common::v1::types::{AuditLogEntryType, MessageSync};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::routes::util::Auth;
use crate::types::UserIdReq;
use crate::ServerState;

use crate::error::{Error, Result};

use super::auth::fetch_auth_state;

/// Check if a user can still login after potentially removing an auth method
/// This prevents users from removing all auth methods which would lock them out
async fn ensure_can_still_login_after_email_removal(
    s: &ServerState,
    user_id: UserId,
) -> Result<()> {
    let mut auth_state = fetch_auth_state(s, user_id).await?;

    // Temporarily "remove" the email auth method to simulate the state after removal
    auth_state.has_email = false;

    // Check if the user can still login with remaining methods
    // According to AuthState::can_login logic:
    // - has_email: if there is at least one verified and primary email address
    // - oauth_providers: if there are any OAuth providers
    // - authenticators: if there are any WebAuthn authenticators (currently not implemented properly)
    if !auth_state.has_email
        && auth_state.oauth_providers.is_empty()
        && auth_state.authenticators.is_empty()
    {
        // Special case: password alone is not sufficient for login according to can_login()
        // A password requires an email to be useful for login (password reset, etc.)
        if auth_state.has_password {
            // If only password remains, they still can't login (based on can_login logic)
            return Err(Error::BadStatic("Cannot remove email: this would lock you out of your account. You must have at least one authentication method remaining."));
        }

        return Err(Error::BadStatic("Cannot remove email: this would lock you out of your account. You must have at least one authentication method remaining."));
    }

    Ok(())
}

/// Email add
#[utoipa::path(
    put,
    path = "/user/{user_id}/email/{addr}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses(
        (status = CREATED, description = "success"),
        (status = OK, description = "already exists"),
    ),
)]
async fn email_add(
    Path((target_user_id_req, email_addr)): Path<(UserIdReq, String)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
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
#[utoipa::path(
    delete,
    path = "/user/{user_id}/email/{addr}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn email_delete(
    Path((target_user_id_req, email)): Path<(UserIdReq, String)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    // we need to keep email addresses in case we need to tell the suspended user anything
    auth.user.ensure_unsuspended()?;

    let email: EmailAddr = email.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let emails = s.data().user_email_list(target_user_id).await?;
    let email_info = emails
        .iter()
        .find(|e| e.email == email)
        .ok_or(Error::NotFound)?;

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
#[utoipa::path(
    get,
    path = "/user/{user_id}/email",
    params(("user_id", description = "User id")),
    tags = ["user_email"],
    responses((status = OK, body = Vec<EmailInfo>, description = "success"))
)]
async fn email_list(
    Path(target_user_id_req): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let emails = s.data().user_email_list(target_user_id).await?;

    Ok(Json(emails).into_response())
}

/// Email update
#[utoipa::path(
    patch,
    path = "/user/{user_id}/email/{addr}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn email_update(
    Path((target_user_id_req, email_addr)): Path<(UserIdReq, String)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<EmailInfoPatch>,
) -> Result<impl IntoResponse> {
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let emails = s.data().user_email_list(target_user_id).await?;
    let email_info = emails
        .iter()
        .find(|e| e.email == email_addr)
        .ok_or(Error::NotFound)?;

    // you can only set an email as primary if it's verified.
    if patch.is_primary == Some(true) {
        if !email_info.is_verified {
            return Err(Error::BadRequest(
                "Email address must be verified to be set as primary.".to_string(),
            ));
        }
    }

    s.data()
        .user_email_update(target_user_id, email_addr.clone(), patch)
        .await?;

    let emails_new = s.data().user_email_list(target_user_id).await?;
    let email_info_new = emails_new
        .iter()
        .find(|e| e.email == email_addr)
        .ok_or(Error::NotFound)?;

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
#[utoipa::path(
    post,
    path = "/user/{user_id}/email/{addr}/resend-verification",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn email_verification_resend(
    Path((target_user_id_req, email_addr)): Path<(UserIdReq, String)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let emails = s.data().user_email_list(target_user_id).await?;
    let email_info = emails
        .into_iter()
        .find(|e| e.email == email_addr)
        .ok_or(Error::NotFound)?;

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
#[utoipa::path(
    post,
    path = "/user/{user_id}/email/{addr}/verify/{code}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
        ("code", description = "Verification code"),
    ),
    tags = ["user_email"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn email_verification_finish(
    Path((target_user_id_req, email_addr, code)): Path<(UserIdReq, String, String)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    s.data()
        .user_email_verify_use(target_user_id, email_addr.clone(), code)
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
        .routes(routes!(email_add))
        .routes(routes!(email_list))
        .routes(routes!(email_delete))
        .routes(routes!(email_update))
        .routes(routes!(email_verification_resend))
        .routes(routes!(email_verification_finish))
}
