use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Form, Json,
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use common::v1::types::{
    application::{Scope, Scopes},
    oauth::{
        Autoconfig, OauthAuthorizeInfo, OauthAuthorizeParams, OauthAuthorizeResponse,
        OauthIntrospectResponse, OauthTokenRequest, OauthTokenResponse, Userinfo,
    },
    util::Time, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, SessionStatus,
    SessionToken, SessionType,
};
use headers::HeaderMapExt;
use http::{HeaderMap, StatusCode};
use sha2::{Digest, Sha256};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    routes::util::{Auth2, HeaderReason},
    types::DbSessionCreate,
    ServerState,
};

use crate::error::{Error, Result};

/// Oauth info
#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tags = ["oauth", "badge.scope.identify"],
    params(OauthAuthorizeParams),
    responses(
        (status = OK, description = "success", body = OauthAuthorizeInfo)
    )
)]
async fn oauth_info(
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<OauthAuthorizeParams>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let app = data.application_get(q.client_id).await?;
    if app.owner_id != auth.user.id && !app.public {
        return Err(Error::NotFound);
    }
    if q.response_type != "code" {
        return Err(Error::BadStatic("unknown response_type"));
    }
    if q.redirect_uri
        .is_none_or(|u| !app.oauth_redirect_uris.iter().any(|a| a == u.as_str()))
    {
        return Err(Error::BadStatic("bad redirect_uri"));
    }
    let mut scopes = HashSet::new();
    for scope in q.scope.split(' ') {
        scopes.insert(Scope::from_str(scope).map_err(|_| Error::BadStatic("invalid scope"))?);
    }
    let auth_user = srv.users.get(auth.user.id, None).await?;
    let bot_user = srv.users.get(app.id.into_inner().into(), None).await?;
    let authorized = if let Ok(existing) = data.connection_get(auth.user.id, app.id).await {
        HashSet::from_iter(existing.scopes).is_superset(&scopes)
    } else {
        false
    };
    Ok(Json(OauthAuthorizeInfo {
        application: app,
        bot_user,
        auth_user,
        authorized,
    }))
}

/// Oauth authorize
#[utoipa::path(
    post,
    path = "/oauth/authorize",
    tags = ["oauth", "badge.scope.identify"],
    params(OauthAuthorizeParams),
    responses(
        (status = OK, description = "success", body = OauthAuthorizeResponse)
    )
)]
async fn oauth_authorize(
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<OauthAuthorizeParams>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let app = data.application_get(q.client_id).await?;
    if app.owner_id != auth.user.id && !app.public {
        return Err(Error::NotFound);
    }
    if q.response_type != "code" {
        return Err(Error::BadStatic("unknown response_type"));
    }

    let redirect_uri = if let Some(uri) = &q.redirect_uri {
        if !app.oauth_redirect_uris.iter().any(|u| u == uri.as_str()) {
            return Err(Error::BadStatic("bad redirect_uri"));
        }
        uri.clone()
    } else {
        app.oauth_redirect_uris
            .get(0)
            .ok_or(Error::BadStatic("no redirect_uri configured"))?
            .parse()?
    };

    let mut scopes = HashSet::new();
    for scope in q.scope.split(' ') {
        scopes.insert(Scope::from_str(scope).map_err(|_| Error::BadStatic("invalid scope"))?);
    }
    let scopes = Scopes(scopes.into_iter().collect());
    data.connection_create(auth.user.id, app.id, scopes.clone())
        .await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason,
        ty: AuditLogEntryType::ConnectionCreate {
            application_id: q.client_id,
            scopes: scopes.clone(),
        },
    })
    .await?;

    let code = Uuid::new_v4().to_string();
    data.oauth_auth_code_create(
        code.clone(),
        app.id,
        auth.user.id,
        redirect_uri.to_string(),
        scopes,
        q.code_challenge,
        q.code_challenge_method,
    )
    .await?;

    let mut redirect_uri = redirect_uri;
    redirect_uri.query_pairs_mut().append_pair("code", &code);
    if let Some(state) = q.state {
        redirect_uri.query_pairs_mut().append_pair("state", &state);
    }

    Ok(Json(OauthAuthorizeResponse { redirect_uri }))
}

/// Oauth exchange token
///
/// exchange an authorization token for an access token
#[utoipa::path(
    post,
    path = "/oauth/token",
    tags = ["oauth"],
    request_body = OauthTokenRequest,
    responses(
        (status = OK, description = "success", body = OauthTokenResponse)
    )
)]
async fn oauth_token(
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    Form(form): Form<OauthTokenRequest>,
) -> Result<impl IntoResponse> {
    let credentials: Option<headers::Authorization<headers::authorization::Basic>> =
        headers.typed_get();
    let client_id = if let Some(client_id) = form.client_id {
        client_id
    } else if let Some(creds) = &credentials {
        creds
            .username()
            .parse()
            .map_err(|_| Error::BadStatic("invalid client_id"))?
    } else {
        return Err(Error::InvalidCredentials);
    };

    let client_secret = if let Some(client_secret) = form.client_secret {
        client_secret
    } else if let Some(creds) = &credentials {
        creds.password().to_string()
    } else {
        return Err(Error::InvalidCredentials);
    };

    let data = s.data();
    let app = data.application_get(client_id).await?;
    if app.id != client_id {
        return Err(Error::InvalidCredentials);
    }

    if app.oauth_confidential {
        if client_secret != app.oauth_secret.unwrap() {
            return Err(Error::InvalidCredentials);
        }
    }

    match form.grant_type.as_str() {
        "authorization_code" => {
            let code = form.code.ok_or(Error::BadStatic("missing code"))?;
            let redirect_uri = form
                .redirect_uri
                .ok_or(Error::BadStatic("missing redirect_uri"))?;

            let (_app_id, user_id, db_redirect_uri, scopes, code_challenge, code_challenge_method) =
                data.oauth_auth_code_use(code).await?;

            if redirect_uri.as_str() != db_redirect_uri {
                return Err(Error::InvalidCredentials);
            }

            if let Some(code_challenge) = code_challenge {
                let code_verifier = form
                    .code_verifier
                    .ok_or(Error::BadStatic("missing code_verifier"))?;
                let method = code_challenge_method.unwrap_or_else(|| "plain".to_string());
                let valid = match method.as_str() {
                    "S256" => {
                        let mut hasher = Sha256::new();
                        hasher.update(code_verifier.as_bytes());
                        let hash = hasher.finalize();
                        let encoded = BASE64_URL_SAFE_NO_PAD.encode(hash);
                        encoded == code_challenge
                    }
                    "plain" => code_verifier == code_challenge,
                    _ => return Err(Error::BadStatic("unsupported code_challenge_method")),
                };
                if !valid {
                    return Err(Error::InvalidCredentials);
                }
            }

            // create a new session for the user
            let token = SessionToken(Uuid::new_v4().to_string());
            let expires_in = 3600; // 1 hour
            let expires_at = Time::now_utc() + Duration::from_secs(expires_in);
            let session = data
                .session_create(DbSessionCreate {
                    token: token.clone(),
                    name: Some(app.name),
                    expires_at: Some(expires_at),
                    ty: SessionType::Access,
                    application_id: Some(app.id),
                })
                .await?;
            data.session_set_status(session.id, SessionStatus::Authorized { user_id })
                .await?;

            let refresh_token_string = Uuid::new_v4().to_string();
            data.oauth_refresh_token_create(refresh_token_string.clone(), session.id)
                .await?;

            let response = OauthTokenResponse {
                access_token: token.0,
                token_type: "Bearer".to_string(),
                expires_in,
                refresh_token: Some(refresh_token_string),
                scope: scopes
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            };

            Ok(Json(response))
        }
        "refresh_token" => {
            let refresh_token = form
                .refresh_token
                .ok_or(Error::BadStatic("missing refresh_token"))?;

            let old_session_id = data.oauth_refresh_token_use(refresh_token).await?;
            let old_session = data.session_get(old_session_id).await?;

            if old_session.app_id != Some(app.id) {
                return Err(Error::InvalidCredentials);
            }

            let user_id = old_session
                .user_id()
                .ok_or(Error::Internal("session has no user".to_string()))?;

            // Invalidate old session
            data.session_delete(old_session_id).await?;
            s.services().sessions.invalidate(old_session_id).await;

            // Create new access token and session
            let token = SessionToken(Uuid::new_v4().to_string());
            let expires_in = 3600; // 1 hour
            let expires_at = Time::now_utc() + Duration::from_secs(expires_in);
            let new_session = data
                .session_create(DbSessionCreate {
                    token: token.clone(),
                    name: Some(app.name.clone()),
                    expires_at: Some(expires_at),
                    ty: SessionType::Access,
                    application_id: Some(app.id),
                })
                .await?;
            data.session_set_status(new_session.id, SessionStatus::Authorized { user_id })
                .await?;

            // Create new refresh token
            let new_refresh_token_string = Uuid::new_v4().to_string();
            data.oauth_refresh_token_create(new_refresh_token_string.clone(), new_session.id)
                .await?;

            let connection = data.connection_get(user_id, app.id).await?;

            let response = OauthTokenResponse {
                access_token: token.0,
                token_type: "Bearer".to_string(),
                expires_in,
                refresh_token: Some(new_refresh_token_string),
                scope: connection
                    .scopes
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            };

            Ok(Json(response))
        }
        _ => Err(Error::BadStatic("unsupported grant_type")),
    }
}

/// Oauth introspect
#[utoipa::path(
    post,
    path = "/oauth/introspect",
    tags = ["oauth", "badge.scope.identify"],
    responses(
        (status = OK, description = "success", body = OauthIntrospectResponse)
    )
)]
async fn oauth_introspect(
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let Some(app_id) = auth.session.app_id else {
        return Err(Error::BadStatic("not an oauth token"));
    };
    let connection = s.data().connection_get(auth.user.id, app_id).await?;
    let res = OauthIntrospectResponse {
        active: true,
        scopes: connection.scopes,
        client_id: app_id,
        username: auth.user.id,
        exp: auth.session.expires_at.map(|e| e.unix_timestamp() as u64),
    };
    Ok(Json(res))
}

/// Oauth revoke
#[utoipa::path(
    post,
    path = "/oauth/revoke",
    tags = ["oauth", "badge.scope.identify"],
    responses(
        (status = NO_CONTENT, description = "success")
    )
)]
async fn oauth_revoke(auth: Auth2, State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    data.session_delete(auth.session.id).await?;
    srv.sessions.invalidate(auth.session.id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Oauth autoconfig
#[utoipa::path(
    get,
    path = "/oauth/.well-known/openid-configuration",
    tags = ["oauth"],
    responses(
        (status = OK, description = "success", body = Autoconfig)
    )
)]
async fn oauth_autoconfig(State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let config = Autoconfig {
        issuer: s.config.api_url.clone(),
        authorization_endpoint: s.config.html_url.join("/authorize")?,
        token_endpoint: s.config.api_url.join("/api/v1/oauth/token")?,
        userinfo_endpoint: s.config.api_url.join("/api/v1/oauth/userinfo")?,
        scopes_supported: vec![
            "identify".to_string(),
            "openid".to_string(),
            "full".to_string(),
            "auth".to_string(),
        ],
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string()],
        subject_types_supported: vec!["public".to_string()],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_post".to_string(),
            "client_secret_basic".to_string(),
        ],
    };
    Ok(Json(config))
}

/// Oauth userinfo
#[utoipa::path(
    get,
    path = "/oauth/userinfo",
    tags = ["oauth", "badge.scope.identify", "badge.scope-opt.email"],
    responses(
        (status = OK, description = "success", body = Userinfo)
    )
)]
async fn oauth_userinfo(
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let user = srv.users.get(auth.user.id, None).await?;
    let email = data
        .user_email_list(auth.user.id)
        .await?
        .into_iter()
        .find(|e| e.is_primary);
    let info = Userinfo {
        iss: srv.state.config.api_url.clone(),
        sub: user.id,
        email: email.clone().map(|e| e.email),
        email_verified: email.map(|e| e.is_verified).unwrap_or_default(),
        name: user.name,
        profile: format!("{}user/{}", srv.state.config.html_url, user.id),
        updated_at: user.version_id.get_timestamp().unwrap().to_unix().0,
        picture: user
            .avatar
            .map(|a| format!("{}media/{}", srv.state.config.cdn_url, a))
            .and_then(|u| u.parse().ok()),
    };
    Ok(Json(info))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(oauth_info))
        .routes(routes!(oauth_authorize))
        .routes(routes!(oauth_token))
        .routes(routes!(oauth_introspect))
        .routes(routes!(oauth_revoke))
        .routes(routes!(oauth_userinfo))
        .routes(routes!(oauth_autoconfig))
}
