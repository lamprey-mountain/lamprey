use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use common::v1::routes;
use common::v1::types::oauth::{AuthServerMetadata, Jwk, OauthGrantType};
use common::v1::types::{
    AuditLogEntryType, SessionStatus, SessionToken, SessionType,
    application::{Application, Scope, Scopes},
    oauth::{
        Jwks, OauthAuthorizeInfo, OauthAuthorizeResponse, OauthIntrospectResponse,
        OauthTokenResponse, OidcClaims, OidcDiscovery, Userinfo,
    },
    util::Time,
};
use headers::HeaderMapExt;
use http::{HeaderMap, StatusCode};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use lamprey_macros::handler;
use sha2::{Digest, Sha256};
use strum::IntoEnumIterator;
use subtle::ConstantTimeEq;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::types::DbSessionCreate;
use crate::{ServerState, routes2};
use common::v1::types::error::{ApiError, ErrorCode};

/// Oauth info
#[handler(routes::oauth_info)]
async fn oauth_info(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::oauth_info::Request,
) -> Result<impl IntoResponse> {
    let (app, _redirect_uri, scopes) = validate_authorize(&s, &auth, &req.params).await?;
    let mut data = s.data();
    let srv = s.services();
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
#[handler(routes::oauth_authorize)]
async fn oauth_authorize(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::oauth_authorize::Request,
) -> Result<impl IntoResponse> {
    let (app, redirect_uri, scopes) = validate_authorize(&s, &auth, &req.params).await?;
    let mut data = s.data();
    let scopes = Scopes(scopes.into_iter().collect());
    data.connection_create(auth.user.id, app.id, scopes.clone())
        .await?;
    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::ConnectionCreate {
        application_id: req.params.client_id,
        scopes: scopes.clone(),
    })
    .await?;

    let code = Uuid::new_v4().to_string();
    data.oauth_auth_code_create(
        code.clone(),
        app.id,
        auth.user.id,
        redirect_uri.to_string(),
        scopes,
        req.params.code_challenge,
        req.params.code_challenge_method,
    )
    .await?;

    let mut redirect_uri = redirect_uri;
    redirect_uri.query_pairs_mut().append_pair("code", &code);
    if let Some(state) = req.params.state {
        redirect_uri.query_pairs_mut().append_pair("state", &state);
    }

    Ok(Json(OauthAuthorizeResponse { redirect_uri }))
}

async fn validate_authorize(
    s: &Arc<ServerState>,
    auth: &Auth,
    q: &common::v1::types::oauth::OauthAuthorizeParams,
) -> Result<(Application, url::Url, HashSet<Scope>)> {
    let mut data = s.data();
    let app = data.application_get(q.client_id).await?;
    if app.owner_id != auth.user.id && !app.public {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownApplication,
        )));
    }
    if q.response_type != "code" {
        return Err(ApiError::from_code(ErrorCode::UnknownResponseType).into());
    }

    let redirect_uri = if let Some(uri) = &q.redirect_uri {
        if !app.oauth_redirect_uris.iter().any(|u| u == uri.as_str()) {
            return Err(ApiError::from_code(ErrorCode::BadRedirectUri).into());
        }
        uri.clone()
    } else {
        app.oauth_redirect_uris
            .get(0)
            .ok_or(ApiError::from_code(ErrorCode::NoRedirectUriConfigured))?
            .parse()?
    };

    let mut scopes = HashSet::new();
    for scope in q.scope.split(' ') {
        scopes.insert(
            Scope::from_str(scope).map_err(|_| ApiError::from_code(ErrorCode::InvalidScope))?,
        );
    }

    Ok((app, redirect_uri, scopes))
}

/// Oauth exchange token
///
/// exchange an authorization token for an access token
#[handler(routes::oauth_token)]
async fn oauth_token(
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    req: routes::oauth_token::Request,
) -> Result<impl IntoResponse> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(ToString::to_string);
    let credentials: Option<headers::Authorization<headers::authorization::Basic>> =
        headers.typed_get();
    let client_id = if let Some(client_id) = req.token.client_id {
        client_id
    } else if let Some(creds) = &credentials {
        creds
            .username()
            .parse()
            .map_err(|_| ApiError::from_code(ErrorCode::InvalidClientId))?
    } else {
        return Err(Error::InvalidCredentials);
    };

    let client_secret = if let Some(client_secret) = req.token.client_secret {
        client_secret
    } else if let Some(creds) = &credentials {
        creds.password().to_string()
    } else {
        return Err(Error::InvalidCredentials);
    };

    let mut data = s.data();
    let app = data.application_get(client_id).await?;
    if app.id != client_id {
        return Err(Error::InvalidCredentials);
    }

    if app.oauth_confidential {
        let stored_secret = app
            .oauth_secret
            .ok_or(Error::Internal("confidential app missing secret".into()))?;
        let secrets_match = client_secret.as_bytes().ct_eq(stored_secret.as_bytes());
        if !bool::from(secrets_match) {
            return Err(Error::InvalidCredentials);
        }
    }

    // TODO: deduplicate/dry up this code
    // maybe extract this logic into a service? (reuse oauth service?)
    match req.token.grant_type.as_str() {
        "authorization_code" => {
            let code = req
                .token
                .code
                .ok_or(ApiError::from_code(ErrorCode::MissingCode))?;
            let redirect_uri = req
                .token
                .redirect_uri
                .ok_or(ApiError::from_code(ErrorCode::MissingRedirectUri))?;

            let (_app_id, user_id, db_redirect_uri, scopes, code_challenge, code_challenge_method) =
                data.oauth_auth_code_use(code).await?;

            if redirect_uri.as_str() != db_redirect_uri {
                return Err(Error::InvalidCredentials);
            }

            if let Some(code_challenge) = code_challenge {
                let code_verifier = req
                    .token
                    .code_verifier
                    .ok_or(ApiError::from_code(ErrorCode::MissingCodeVerifier))?;
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
                    _ => {
                        return Err(
                            ApiError::from_code(ErrorCode::UnsupportedCodeChallengeMethod).into(),
                        );
                    }
                };
                if !valid {
                    return Err(Error::InvalidCredentials);
                }
            }

            let token = SessionToken(Uuid::new_v4().to_string());
            let expires_in = 3600;
            let expires_at = Time::now_utc() + Duration::from_secs(expires_in);
            let session = data
                .session_create(DbSessionCreate {
                    token: token.clone(),
                    name: Some(app.name),
                    expires_at: Some(expires_at),
                    ty: SessionType::Access,
                    application_id: Some(app.id),
                    ip_addr: None,
                    user_agent: user_agent.clone(),
                })
                .await?;
            data.session_set_status(session.id, SessionStatus::Authorized { user_id })
                .await?;

            let refresh_token_string = Uuid::new_v4().to_string();
            data.oauth_refresh_token_create(refresh_token_string.clone(), session.id)
                .await?;

            let mut id_token = None;
            let scope_list = Scopes(scopes.clone().into_iter().collect());
            if scope_list.has(&Scope::Openid) {
                // PERF: don't (re)load keys every time
                let srv = s.services();
                let c = srv.config.internal_get().await?;
                let jwk: jsonwebkey::JsonWebKey = serde_json::from_str(&c.oidc_jwk_key)?;
                let pem = jwk.key.to_pem();

                let (encoding_key, alg) = match jwk.algorithm {
                    Some(jsonwebkey::Algorithm::ES256) => {
                        (EncodingKey::from_ec_pem(pem.as_bytes())?, Algorithm::ES256)
                    }
                    Some(jsonwebkey::Algorithm::RS256) => {
                        (EncodingKey::from_rsa_pem(pem.as_bytes())?, Algorithm::RS256)
                    }
                    _ => return Err(Error::Internal("unsupported signing alg".into())),
                };

                let header = Header {
                    alg,
                    kid: jwk.key_id.clone(),
                    ..Default::default()
                };

                let claims = OidcClaims {
                    iss: s.config.api_url.to_string(),
                    sub: user_id.to_string(),
                    aud: client_id.to_string(),
                    exp: expires_at.unix_timestamp() as u64,
                    iat: Time::now_utc().unix_timestamp() as u64,
                    nonce: None,
                };

                id_token = Some(encode(&header, &claims, &encoding_key)?);
            }

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
                id_token,
            };

            Ok(Json(response))
        }
        "refresh_token" => {
            let refresh_token = req
                .token
                .refresh_token
                .ok_or(ApiError::from_code(ErrorCode::MissingRefreshToken))?;

            let old_session_id = data.oauth_refresh_token_use(refresh_token).await?;
            let old_session = data.session_get(old_session_id).await?;

            if old_session.app_id != Some(app.id) {
                return Err(Error::InvalidCredentials);
            }

            let user_id = old_session
                .user_id()
                .ok_or(Error::Internal("session has no user".to_string()))?;

            data.session_delete(old_session_id).await?;
            s.services().sessions.invalidate(old_session_id).await;

            let token = SessionToken(Uuid::new_v4().to_string());
            let expires_in = 3600;
            let expires_at = Time::now_utc() + Duration::from_secs(expires_in);
            let new_session = data
                .session_create(DbSessionCreate {
                    token: token.clone(),
                    name: Some(app.name.clone()),
                    expires_at: Some(expires_at),
                    ty: SessionType::Access,
                    application_id: Some(app.id),
                    ip_addr: None,
                    user_agent: user_agent.clone(),
                })
                .await?;
            data.session_set_status(new_session.id, SessionStatus::Authorized { user_id })
                .await?;

            let new_refresh_token_string = Uuid::new_v4().to_string();
            data.oauth_refresh_token_create(new_refresh_token_string.clone(), new_session.id)
                .await?;

            let connection = data.connection_get(user_id, app.id).await?;

            let mut id_token = None;
            let scope_list = Scopes(connection.scopes.clone().into_iter().collect());
            if scope_list.has(&Scope::Openid) {
                // PERF: don't (re)load keys every time
                let srv = s.services();
                let c = srv.config.internal_get().await?;
                let jwk: jsonwebkey::JsonWebKey = serde_json::from_str(&c.oidc_jwk_key)?;
                let pem = jwk.key.to_pem();

                let (encoding_key, alg) = match jwk.algorithm {
                    Some(jsonwebkey::Algorithm::ES256) => {
                        (EncodingKey::from_ec_pem(pem.as_bytes())?, Algorithm::ES256)
                    }
                    Some(jsonwebkey::Algorithm::RS256) => {
                        (EncodingKey::from_rsa_pem(pem.as_bytes())?, Algorithm::RS256)
                    }
                    _ => return Err(Error::Internal("unsupported signing alg".into())),
                };

                let header = Header {
                    alg,
                    kid: jwk.key_id.clone(),
                    ..Default::default()
                };

                let claims = OidcClaims {
                    iss: s.config.api_url.to_string(),
                    sub: user_id.to_string(),
                    aud: app.id.to_string(),
                    exp: expires_at.unix_timestamp() as u64,
                    iat: Time::now_utc().unix_timestamp() as u64,
                    nonce: None,
                };

                id_token = Some(encode(&header, &claims, &encoding_key)?);
            }

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
                id_token,
            };

            Ok(Json(response))
        }
        _ => Err(ApiError::from_code(ErrorCode::UnsupportedGrantType).into()),
    }
}

/// Oauth introspect
#[handler(routes::oauth_introspect)]
async fn oauth_introspect(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_introspect::Request,
) -> Result<impl IntoResponse> {
    let Some(app_id) = auth.session.app_id else {
        return Err(ApiError::from_code(ErrorCode::NotAnOauthToken).into());
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
#[handler(routes::oauth_revoke)]
async fn oauth_revoke(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_revoke::Request,
) -> Result<impl IntoResponse> {
    let mut data = s.data();
    let srv = s.services();
    data.session_delete(auth.session.id).await?;
    srv.sessions.invalidate(auth.session.id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Oauth autoconfig auth server
#[handler(routes::oauth_autoconfig_auth_server)]
async fn oauth_autoconfig_auth_server(
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_autoconfig_auth_server::Request,
) -> Result<impl IntoResponse> {
    let config = AuthServerMetadata {
        issuer: s.config.api_url.clone(),
        authorization_endpoint: s.config.html_url.join("/authorize")?,
        token_endpoint: s.config.api_url.join("/api/v1/oauth/token")?,
        jwks_uri: s.config.api_url.join("/.well-known/jwks.json")?,
        scopes_supported: Scope::iter().map(|a| a.to_string()).collect(),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string()],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_post".to_string(),
            "client_secret_basic".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string(), "plain".to_string()],
    };
    Ok(Json(config))
}

/// Oauth autoconfig OIDC
#[handler(routes::oauth_autoconfig_openid)]
async fn oauth_autoconfig_openid(
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_autoconfig_openid::Request,
) -> Result<impl IntoResponse> {
    let config = OidcDiscovery {
        issuer: s.config.api_url.clone(),
        authorization_endpoint: s.config.html_url.join("/authorize")?,
        token_endpoint: s.config.api_url.join("/api/v1/oauth/token")?,
        userinfo_endpoint: s.config.api_url.join("/api/v1/oauth/userinfo")?,
        jwks_uri: s.config.api_url.join("/.well-known/jwks.json")?,
        scopes_supported: Scope::iter().map(|a| a.to_string()).collect(),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string()],
        subject_types_supported: vec!["public".to_string()],
        id_token_signing_alg_values_supported: vec!["ES256".to_string()],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_post".to_string(),
            "client_secret_basic".to_string(),
        ],
        claims_supported: vec![
            "iss".to_string(),
            "sub".to_string(),
            "name".to_string(),
            "email".to_string(),
            "email_verified".to_string(),
            "picture".to_string(),
            "updated_at".to_string(),
            "profile".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string(), "plain".to_string()],
    };
    Ok(Json(config))
}

/// Oauth JWKS
#[handler(routes::oauth_jwks)]
async fn oauth_jwks(
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_jwks::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let c = srv.config.internal_get().await?;
    // NOTE: this json is originally a jsonwebkey::JsonWebKey
    let jwk: Jwk = serde_json::from_str(&c.oidc_jwk_key)?;
    let jwks = Jwks { keys: vec![jwk] };
    Ok(Json(jwks))
}

/// Oauth userinfo
#[handler(routes::oauth_userinfo)]
async fn oauth_userinfo(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_userinfo::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let mut data = s.data();

    // NOTE: currently i assume every session has Identify
    // in the future, i'll either require every client to have Identify or require Identify to access this endpoint

    let user = srv.users.get(auth.user.id, None).await?;

    let has_email_scope = auth.scopes.has(&Scope::Email);
    let email = if has_email_scope {
        data.user_email_list(auth.user.id)
            .await?
            .into_iter()
            .find(|e| e.is_primary)
    } else {
        None
    };

    let info = Userinfo {
        iss: srv.state.config().api_url.clone(),
        sub: user.id,
        email: email.clone().map(|e| e.email),
        email_verified: email.map(|e| e.is_verified).unwrap_or_default(),
        name: user.name,
        profile: format!("{}user/{}", srv.state.config().html_url, user.id),
        updated_at: user.version_id.get_timestamp().unwrap().to_unix().0,
        picture: user
            .avatar
            .map(|a| format!("{}media/{}", srv.state.config().cdn_url, a))
            .and_then(|u| u.parse().ok()),
    };
    Ok(Json(info))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(oauth_info))
        .routes(routes2!(oauth_authorize))
        .routes(routes2!(oauth_token))
        .routes(routes2!(oauth_introspect))
        .routes(routes2!(oauth_revoke))
        .routes(routes2!(oauth_userinfo))
        .routes(routes2!(oauth_autoconfig_auth_server))
        .routes(routes2!(oauth_autoconfig_openid))
        .routes(routes2!(oauth_jwks))
}
