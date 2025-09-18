use std::sync::Arc;

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::types::email::{EmailAddr, EmailInfo, EmailInfoPatch};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use crate::error::{Error, Result};

use super::util::Auth;

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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
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

            s.services
                .email
                .send(
                    email_addr,
                    "Verify your email address".to_string(),
                    format!("Your verification code is: {}", code),
                    None,
                )
                .await?;

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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    // we need to keep email addresses in case we need to tell the suspended user anything
    auth_user.ensure_unsuspended()?;

    let email: EmailAddr = email.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }

    s.data().user_email_delete(target_user_id, email).await?;

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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<EmailInfoPatch>,
) -> Result<impl IntoResponse> {
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }

    // You can only set an email as primary if it's verified.
    if patch.is_primary == Some(true) {
        let emails = s.data().user_email_list(target_user_id).await?;
        let email_info = emails
            .iter()
            .find(|e| e.email == email_addr)
            .ok_or(Error::NotFound)?;
        if !email_info.is_verified {
            return Err(Error::BadRequest(
                "Email address must be verified to be set as primary.".to_string(),
            ));
        }
    }

    s.data()
        .user_email_update(target_user_id, email_addr, patch)
        .await?;

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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
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

    s.services
        .email
        .send(
            email_addr,
            "Verify your email address".to_string(),
            format!("Your verification code is: {}", code),
            None,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let email_addr: EmailAddr = email_addr.try_into()?;
    let target_user_id = match target_user_id_req {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }

    s.data()
        .user_email_verify_use(target_user_id, email_addr, code)
        .await?;

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
