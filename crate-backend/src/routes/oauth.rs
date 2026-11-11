use std::{
    collections::HashSet,
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use common::v1::routes;
use common::v1::types::oauth::{
    AuthServerMetadata, Jwk, OauthAuthMethod, OauthAuthorizeParams, OauthClientRegistrationRequest,
    OauthCodeChallengeMethod, OauthDeviceAuthorizationResponse, OauthDeviceInfo, OauthGrantType,
    OauthResponseType,
};
use common::v1::types::{
    ApplicationId, AuditLogEntryType, DYNAMIC_APP_OWNER_ID, SessionStatus, SessionToken,
    SessionType, UserId,
    application::{Application, Scope, Scopes},
    oauth::{
        Jwks, OauthAuthorizeInfo, OauthAuthorizeResponse, OauthClientRegistrationResponse,
        OauthIntrospectResponse, OauthTokenResponse, OidcClaims, OidcDiscovery, Userinfo,
    },
    util::{Changes, Time},
};
use headers::HeaderMapExt;
use http::{HeaderMap, StatusCode};
use kerosene_services::services::oauth2::device_authorization::{
    DEVICE_CODE_EXPIRATION, DEVICE_CODE_INTERVAL, DeviceAuthorizationPoll,
};
use lamprey_macros::handler;
use sha2::{Digest, Sha256};
use strum::IntoEnumIterator;
use subtle::ConstantTimeEq;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::routes::util::audit::{SessionlessAuditContext, audit_log_sessionless};
use crate::routes::util::headers::HeadersRequest;
use crate::types::{DbSessionCreate, DbUserCreate};
use crate::{ServerState, routes2};
use common::v1::types::error::{ApiError, ErrorCode};

/// a validated dynamic client registration request
struct NormalizedRegistration {
    client_name: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<OauthGrantType>,
    response_types: Vec<OauthResponseType>,
    scope: Option<String>,
}

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
    // TODO: skip this if application is not a bot
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

/// Oauth register
#[handler(routes::oauth_register)]
async fn oauth_register(
    State(s): State<Arc<ServerState>>,
    headers: HeadersRequest,
    req: routes::oauth_register::Request,
) -> Result<impl IntoResponse> {
    validate_registration(&s, &headers)?;
    let NormalizedRegistration {
        client_name,
        redirect_uris,
        grant_types,
        response_types,
        scope,
    } = normalize_registration(req.registration)?;

    let mut txn = s.acquire_data().await?;
    let user = txn
        .user_create(DbUserCreate {
            id: None,
            parent_id: Some(DYNAMIC_APP_OWNER_ID),
            name: client_name.clone(),
            description: None,
            puppet: None,
            registered_at: Some(Time::now_utc()),
            system: false,
            bot: true,
            remote: None,
        })
        .await?;

    let app = Application {
        id: user.id.into_inner().into(),
        owner_id: DYNAMIC_APP_OWNER_ID,
        name: client_name.clone(),
        description: None,
        bridge: None,
        public: true,
        oauth_secret: None,
        oauth_redirect_uris: redirect_uris.clone(),
        oauth_confidential: false,
    };
    txn.application_insert(app.clone()).await?;
    txn.commit().await?;

    audit_log_sessionless(
        &s,
        SessionlessAuditContext {
            room_id: app.id.into_inner().into(),
            user_id: DYNAMIC_APP_OWNER_ID,
            reason: headers.reason.clone(),
            ip_addr: headers.ip_addr.map(|ip| ip.to_string()),
            user_agent: headers.user_agent.clone(),
            application_id: None,
        },
        AuditLogEntryType::ApplicationCreate {
            application_id: app.id,
            changes: Changes::new()
                .add("name", &app.name)
                .add("description", &app.description)
                .add("public", &app.public)
                .add("oauth_redirect_uris", &app.oauth_redirect_uris)
                .add("oauth_confidential", &app.oauth_confidential)
                .build(),
        },
    )
    .await?;

    let response = OauthClientRegistrationResponse {
        client_id: app.id,
        client_id_issued_at: Time::now_utc().unix_timestamp() as u64,
        client_name,
        redirect_uris,
        token_endpoint_auth_method: OauthAuthMethod::None,
        grant_types,
        response_types,
        scope,
    };
    let mut response = (StatusCode::CREATED, Json(response)).into_response();
    response
        .headers_mut()
        .typed_insert(headers::CacheControl::new().with_no_store());
    response.headers_mut().insert(
        http::header::PRAGMA,
        http::HeaderValue::from_static("no-cache"),
    );
    Ok(response)
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
        auth.session.authorized_at,
    )
    .await?;

    let mut redirect_uri = redirect_uri;
    redirect_uri.query_pairs_mut().append_pair("code", &code);
    if let Some(state) = req.params.state {
        redirect_uri.query_pairs_mut().append_pair("state", &state);
    }
    redirect_uri
        .query_pairs_mut()
        .append_pair("iss", s.config.api_url.as_str());

    Ok(Json(OauthAuthorizeResponse { redirect_uri }))
}

fn parse_client_credentials(
    headers: &HeaderMap,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Result<(ApplicationId, Option<String>)> {
    let credentials: Option<headers::Authorization<headers::authorization::Basic>> =
        headers.typed_get();

    let client_id = if let Some(client_id) = client_id {
        client_id
    } else if let Some(credentials) = &credentials {
        credentials.username().to_string()
    } else {
        return Err(Error::InvalidCredentials);
    };
    let client_id = client_id
        .parse()
        .map_err(|_| ApiError::from_code(ErrorCode::InvalidClientId))?;
    let client_secret = client_secret.or_else(|| credentials.map(|c| c.password().to_string()));

    Ok((client_id, client_secret))
}

fn verify_client_secret(app: &Application, client_secret: Option<String>) -> Result<()> {
    if app.oauth_confidential {
        let client_secret = client_secret.ok_or(Error::InvalidCredentials)?;
        let stored_secret = app
            .oauth_secret
            .clone()
            .ok_or(Error::Internal("confidential app missing secret".into()))?;
        let secrets_match = client_secret.as_bytes().ct_eq(stored_secret.as_bytes());
        if !bool::from(secrets_match) {
            return Err(Error::InvalidCredentials);
        }
    }
    Ok(())
}

fn parse_scopes(scope: &str) -> Result<Scopes> {
    let mut scopes = HashSet::new();
    for scope in scope.split_whitespace() {
        scopes.insert(
            Scope::from_str(scope).map_err(|_| ApiError::from_code(ErrorCode::InvalidScope))?,
        );
    }
    Ok(Scopes(scopes.into_iter().collect()))
}

fn oauth_error_with_status(
    status: StatusCode,
    error: &'static str,
    description: &'static str,
) -> Error {
    let body = serde_json::json!({
        "error": error,
        "error_description": description,
    });
    let mut builder = http::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json");
    if status == StatusCode::UNAUTHORIZED {
        builder = builder.header(http::header::WWW_AUTHENTICATE, "Bearer");
    }
    Error::Response(
        builder
            .body(bytes::Bytes::from(
                serde_json::to_vec(&body).expect("oauth error should serialize"),
            ))
            .expect("oauth error response should build"),
    )
}

fn oauth_error(error: &'static str, description: &'static str) -> Error {
    oauth_error_with_status(StatusCode::BAD_REQUEST, error, description)
}

fn validate_registration(s: &Arc<ServerState>, headers: &HeadersRequest) -> Result<()> {
    let config = &s.config.oauth_server;
    if !config.dynamic_registration {
        return Err(oauth_error_with_status(
            StatusCode::NOT_FOUND,
            "registration_disabled",
            "dynamic client registration is disabled",
        ));
    }

    if let Some(expected) = &config.registration_token {
        let expected = expected
            .load()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let valid = headers
            .authorization
            .as_ref()
            .is_some_and(|auth| bool::from(auth.token().as_bytes().ct_eq(expected.as_bytes())));
        if !valid {
            return Err(oauth_error_with_status(
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                "a valid registration token is required",
            ));
        }
    }

    Ok(())
}

fn validate_redirect_uris(uris: &[String]) -> Result<()> {
    if uris.len() > 8 {
        return Err(oauth_error(
            "invalid_redirect_uri",
            "at most 8 redirect uris are allowed",
        ));
    }

    for uri in uris {
        let url = url::Url::parse(uri).map_err(|_| {
            oauth_error(
                "invalid_redirect_uri",
                "redirect uri must be an absolute url",
            )
        })?;
        if url.fragment().is_some() {
            return Err(oauth_error(
                "invalid_redirect_uri",
                "redirect uri must not contain a fragment",
            ));
        }

        match url.scheme() {
            "https" => {}
            "http" => {
                let loopback = match url.host() {
                    Some(url::Host::Domain("localhost")) => true,
                    Some(url::Host::Ipv4(ip)) => ip == Ipv4Addr::LOCALHOST,
                    Some(url::Host::Ipv6(ip)) => ip == Ipv6Addr::LOCALHOST,
                    _ => false,
                };
                if !loopback {
                    return Err(oauth_error(
                        "invalid_redirect_uri",
                        "http redirect uris are only allowed for loopback hosts",
                    ));
                }
            }
            "javascript" | "data" | "file" | "vbscript" | "about" | "blob" => {
                return Err(oauth_error(
                    "invalid_redirect_uri",
                    "unsupported redirect uri scheme",
                ));
            }
            // private-use scheme for native apps (RFC 8252)
            _ => {}
        }
    }

    Ok(())
}

fn normalize_registration(req: OauthClientRegistrationRequest) -> Result<NormalizedRegistration> {
    if req
        .token_endpoint_auth_method
        .is_some_and(|method| method != OauthAuthMethod::None)
    {
        return Err(oauth_error(
            "invalid_client_metadata",
            "only public clients are supported",
        ));
    }

    let grant_types = req
        .grant_types
        .unwrap_or_else(|| vec![OauthGrantType::AuthorizationCode]);
    if grant_types.is_empty() {
        return Err(oauth_error(
            "invalid_client_metadata",
            "grant_types must not be empty",
        ));
    }
    for grant_type in &grant_types {
        if !matches!(
            grant_type,
            OauthGrantType::AuthorizationCode
                | OauthGrantType::RefreshToken
                | OauthGrantType::DeviceCode
        ) {
            return Err(oauth_error(
                "invalid_client_metadata",
                "unsupported grant type",
            ));
        }
    }

    let response_types = req
        .response_types
        .unwrap_or_else(|| vec![OauthResponseType::Code]);
    if response_types.is_empty()
        || response_types
            .iter()
            .any(|response_type| *response_type != OauthResponseType::Code)
    {
        return Err(oauth_error(
            "invalid_client_metadata",
            "only the code response type is supported",
        ));
    }

    if grant_types.contains(&OauthGrantType::AuthorizationCode) && req.redirect_uris.is_empty() {
        return Err(oauth_error(
            "invalid_redirect_uri",
            "authorization_code requires at least one redirect uri",
        ));
    }
    validate_redirect_uris(&req.redirect_uris)?;

    let client_name = req
        .client_name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "OAuth Client".to_string());
    if client_name.chars().count() > 64 {
        return Err(oauth_error(
            "invalid_client_metadata",
            "client_name must be 64 characters or fewer",
        ));
    }

    let scope = match req.scope.as_deref().map(str::trim) {
        Some(scope) if !scope.is_empty() => {
            parse_scopes(scope)
                .map_err(|_| oauth_error("invalid_client_metadata", "invalid scope requested"))?;
            Some(scope.to_string())
        }
        _ => None,
    };

    Ok(NormalizedRegistration {
        client_name,
        redirect_uris: req.redirect_uris,
        grant_types,
        response_types,
        scope,
    })
}

fn validate_pkce(app: &Application, q: &OauthAuthorizeParams) -> Result<()> {
    let requires_pkce = app.is_dynamic_oauth_client() || !app.oauth_confidential;
    let has_challenge = q
        .code_challenge
        .as_deref()
        .is_some_and(|challenge| !challenge.is_empty());
    if !has_challenge {
        if !requires_pkce {
            return Ok(());
        }
        return Err(ApiError::from_code(ErrorCode::MissingCodeChallenge).into());
    }

    let method =
        OauthCodeChallengeMethod::from_str(q.code_challenge_method.as_deref().unwrap_or("plain"))
            .map_err(|_| ApiError::from_code(ErrorCode::UnsupportedCodeChallengeMethod))?;

    if requires_pkce && method != OauthCodeChallengeMethod::S256 {
        return Err(ApiError::from_code(ErrorCode::UnsupportedCodeChallengeMethod).into());
    }

    Ok(())
}

async fn validate_authorize(
    s: &Arc<ServerState>,
    auth: &Auth,
    q: &OauthAuthorizeParams,
) -> Result<(Application, url::Url, HashSet<Scope>)> {
    let mut data = s.data();
    let app = data.application_get(q.client_id).await?;
    if app.owner_id != auth.user.id && !app.public {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownApplication,
        )));
    }
    if q.response_type != OauthResponseType::Code {
        return Err(ApiError::from_code(ErrorCode::UnknownResponseType).into());
    }

    validate_pkce(&app, q)?;

    if let Some(max_age) = q.max_age {
        let authorized_at = auth
            .session
            .authorized_at
            .ok_or_else(|| ApiError::from_code(ErrorCode::LoginRequired))?;
        let age = Time::now_utc() - authorized_at;
        if max_age == 0 || age.whole_seconds() > max_age as i64 {
            return Err(ApiError::from_code(ErrorCode::LoginRequired).into());
        }
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

    Ok((app.strip_private(), redirect_uri, scopes))
}

/// Oauth device authorization
#[handler(routes::oauth_device_authorization)]
async fn oauth_device_authorization(
    State(s): State<Arc<ServerState>>,
    headers: HeaderMap,
    req: routes::oauth_device_authorization::Request,
) -> Result<impl IntoResponse> {
    let (client_id, client_secret) =
        parse_client_credentials(&headers, Some(req.form.client_id), req.form.client_secret)?;

    let mut data = s.data();
    let app = data.application_get(client_id).await?;
    verify_client_secret(&app, client_secret)?;

    let scopes = match req.form.scope.as_deref() {
        Some(scope) if !scope.is_empty() => parse_scopes(scope)?,
        _ => Scopes(vec![Scope::Identify]),
    };

    let (device_code, authorization) = s
        .services()
        .oauth
        .device_authorizations
        .create(app.id, scopes);

    let verification_uri = s.config.html_url.join("/authorize-device")?;
    let mut verification_uri_complete = verification_uri.clone();
    verification_uri_complete
        .query_pairs_mut()
        .append_pair("user_code", &authorization.user_code);

    Ok(Json(OauthDeviceAuthorizationResponse {
        device_code,
        user_code: authorization.user_code,
        verification_uri,
        verification_uri_complete,
        expires_in: DEVICE_CODE_EXPIRATION.as_secs(),
        interval: DEVICE_CODE_INTERVAL.as_secs(),
    }))
}

/// Oauth device info
#[handler(routes::oauth_device_info)]
async fn oauth_device_info(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::oauth_device_info::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let authorization = srv
        .oauth
        .device_authorizations
        .get_by_user_code(&req.query.user_code)
        .ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::InvalidData,
                "invalid or expired user code".to_string(),
            )
        })?;

    let mut data = s.data();
    let app = data.application_get(authorization.application_id).await?;
    if app.owner_id != auth.user.id && !app.public {
        return Err(ApiError::from_code(ErrorCode::UnknownApplication).into());
    }

    let auth_user = srv.users.get(auth.user.id, None).await?;
    let bot_user = srv.users.get(app.id.into_inner().into(), None).await?;

    Ok(Json(OauthDeviceInfo {
        application: app.strip_private(),
        bot_user,
        auth_user,
        scopes: authorization.scopes,
    }))
}

/// Oauth device authorize
#[handler(routes::oauth_device_authorize)]
async fn oauth_device_authorize(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::oauth_device_authorize::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let authorization = srv
        .oauth
        .device_authorizations
        .get_by_user_code(&req.decision.user_code)
        .ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::InvalidData,
                "invalid or expired user code".to_string(),
            )
        })?;

    let mut data = s.data();
    let app = data.application_get(authorization.application_id).await?;
    if app.owner_id != auth.user.id && !app.public {
        return Err(ApiError::from_code(ErrorCode::UnknownApplication).into());
    }

    if req.decision.approve {
        data.connection_create(auth.user.id, app.id, authorization.scopes.clone())
            .await?;
        let al = auth.audit_log(auth.user.id.into_inner().into());
        al.commit_success(AuditLogEntryType::ConnectionCreate {
            application_id: app.id,
            scopes: authorization.scopes.clone(),
        })
        .await?;
    }

    srv.oauth.device_authorizations.authorize(
        &req.decision.user_code,
        auth.user.id,
        req.decision.approve,
    )?;

    Ok(StatusCode::NO_CONTENT)
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
    let (client_id, client_secret) =
        parse_client_credentials(&headers, req.token.client_id, req.token.client_secret)?;

    let mut data = s.data();
    let app = data.application_get(client_id).await?;
    if app.id != client_id {
        return Err(Error::InvalidCredentials);
    }

    verify_client_secret(&app, client_secret)?;

    // TODO: deduplicate/dry up this code
    // maybe extract this logic into a service? (reuse oauth service?)
    match req.token.grant_type {
        OauthGrantType::AuthorizationCode => {
            let code = req
                .token
                .code
                .ok_or(ApiError::from_code(ErrorCode::MissingCode))?;
            let redirect_uri = req
                .token
                .redirect_uri
                .ok_or(ApiError::from_code(ErrorCode::MissingRedirectUri))?;

            let (
                _app_id,
                user_id,
                db_redirect_uri,
                scopes,
                code_challenge,
                code_challenge_method,
                auth_time,
            ) = data.oauth_auth_code_use(code).await?;

            if redirect_uri.as_str() != db_redirect_uri {
                return Err(Error::InvalidCredentials);
            }

            if let Some(code_challenge) = code_challenge {
                let code_verifier = req
                    .token
                    .code_verifier
                    .ok_or(ApiError::from_code(ErrorCode::MissingCodeVerifier))?;
                // NOTE: maybe make db handle from_str for this?
                let method = OauthCodeChallengeMethod::from_str(
                    &code_challenge_method.unwrap_or_else(|| "plain".to_string()),
                )
                .map_err(|_| ApiError::from_code(ErrorCode::UnsupportedCodeChallengeMethod))?;
                let valid = match method {
                    OauthCodeChallengeMethod::S256 => {
                        let mut hasher = Sha256::new();
                        hasher.update(code_verifier.as_bytes());
                        let hash = hasher.finalize();
                        let encoded = BASE64_URL_SAFE_NO_PAD.encode(hash);
                        encoded == code_challenge
                    }
                    OauthCodeChallengeMethod::Plain => code_verifier == code_challenge,
                };
                if !valid {
                    return Err(ApiError::from_code(ErrorCode::InvalidCodeVerifier).into());
                }
            } else if !app.oauth_confidential {
                return Err(ApiError::from_code(ErrorCode::MissingCodeChallenge).into());
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
                let srv = s.services();

                let claims = OidcClaims {
                    iss: s.config.api_url.to_string(),
                    sub: user_id.to_string(),
                    aud: client_id.to_string(),
                    exp: expires_at.unix_timestamp() as u64,
                    iat: Time::now_utc().unix_timestamp() as u64,
                    auth_time: auth_time.map(|time| time.unix_timestamp() as u64),
                    nonce: None,
                };

                id_token = Some(srv.oauth_provider.sign_oidc_claims(&claims).await?);
            }

            let response = OauthTokenResponse {
                access_token: token.0,
                token_type: "Bearer".to_string(),
                expires_in: Some(expires_in),
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
        OauthGrantType::ClientCredentials => {
            if !app.oauth_confidential {
                return Err(Error::BadStatic(
                    "client credentials grant requires a confidential client".into(),
                ));
            }

            let scopes = match req.token.scope.as_deref() {
                Some(scope) if !scope.is_empty() => parse_scopes(scope)?,
                _ => Scopes(vec![Scope::Full]),
            };

            // the bot user acts on behalf of the application
            let bot_user_id = UserId::from(app.id.into_inner());
            data.connection_create(bot_user_id, app.id, scopes.clone())
                .await?;

            let token = SessionToken(Uuid::new_v4().to_string());
            let expires_in = 3600;
            let expires_at = Time::now_utc() + Duration::from_secs(expires_in);
            let session = data
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
            data.session_set_status(
                session.id,
                SessionStatus::Authorized {
                    user_id: bot_user_id,
                },
            )
            .await?;

            let response = OauthTokenResponse {
                access_token: token.0,
                token_type: "Bearer".to_string(),
                expires_in: Some(expires_in),
                refresh_token: None,
                scope: scopes
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                id_token: None,
            };

            Ok(Json(response))
        }
        OauthGrantType::DeviceCode => {
            let device_code = req
                .token
                .device_code
                .ok_or_else(|| oauth_error("invalid_request", "missing device_code"))?;

            match s.services().oauth.device_authorizations.poll(&device_code) {
                DeviceAuthorizationPoll::Pending => {
                    return Err(oauth_error(
                        "authorization_pending",
                        "the user has not approved the request yet",
                    ));
                }
                DeviceAuthorizationPoll::SlowDown => {
                    return Err(oauth_error("slow_down", "polling too frequently"));
                }
                DeviceAuthorizationPoll::Denied => {
                    return Err(oauth_error("access_denied", "the user denied the request"));
                }
                DeviceAuthorizationPoll::Expired => {
                    return Err(oauth_error("expired_token", "the device code has expired"));
                }
                DeviceAuthorizationPoll::Invalid => {
                    return Err(oauth_error("invalid_grant", "invalid device code"));
                }
                DeviceAuthorizationPoll::Approved {
                    application_id,
                    user_id,
                    scopes,
                } => {
                    let result: Result<OauthTokenResponse> = async {
                        if application_id != app.id {
                            return Err(oauth_error(
                                "invalid_grant",
                                "device code belongs to another client",
                            ));
                        }

                        data.connection_create(user_id, app.id, scopes.clone())
                            .await?;

                        let token = SessionToken(Uuid::new_v4().to_string());
                        let expires_in = 3600;
                        let expires_at = Time::now_utc() + Duration::from_secs(expires_in);
                        let session = data
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
                        data.session_set_status(session.id, SessionStatus::Authorized { user_id })
                            .await?;

                        let refresh_token_string = Uuid::new_v4().to_string();
                        data.oauth_refresh_token_create(refresh_token_string.clone(), session.id)
                            .await?;

                        let mut id_token = None;
                        if scopes.has(&Scope::Openid) {
                            let srv = s.services();
                            let claims = OidcClaims {
                                iss: s.config.api_url.to_string(),
                                sub: user_id.to_string(),
                                aud: client_id.to_string(),
                                exp: expires_at.unix_timestamp() as u64,
                                iat: Time::now_utc().unix_timestamp() as u64,
                                auth_time: None,
                                nonce: None,
                            };
                            id_token = Some(srv.oauth_provider.sign_oidc_claims(&claims).await?);
                        }

                        Ok(OauthTokenResponse {
                            access_token: token.0,
                            token_type: "Bearer".to_string(),
                            expires_in: Some(expires_in),
                            refresh_token: Some(refresh_token_string),
                            scope: scopes
                                .into_iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>()
                                .join(" "),
                            id_token,
                        })
                    }
                    .await;

                    let devices = s.services().oauth.device_authorizations.clone();
                    match result {
                        Ok(response) => {
                            devices.consume(&device_code);
                            Ok(Json(response))
                        }
                        Err(error) => {
                            devices.release(&device_code);
                            Err(error)
                        }
                    }
                }
            }
        }
        OauthGrantType::RefreshToken => {
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
                let srv = s.services();

                let claims = OidcClaims {
                    iss: s.config.api_url.to_string(),
                    sub: user_id.to_string(),
                    aud: app.id.to_string(),
                    exp: expires_at.unix_timestamp() as u64,
                    iat: Time::now_utc().unix_timestamp() as u64,
                    auth_time: None,
                    nonce: None,
                };

                id_token = Some(srv.oauth_provider.sign_oidc_claims(&claims).await?);
            }

            let response = OauthTokenResponse {
                access_token: token.0,
                token_type: "Bearer".to_string(),
                expires_in: Some(expires_in),
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
        } // _ => Err(ApiError::from_code(ErrorCode::UnsupportedGrantType).into()),
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

fn registration_endpoint(s: &ServerState) -> Result<Option<url::Url>> {
    if !s.config.oauth_server.dynamic_registration {
        return Ok(None);
    }
    Ok(Some(s.config.api_url.join("/api/v1/oauth/register")?))
}

/// Oauth autoconfig auth server
#[handler(routes::oauth_autoconfig_auth_server)]
async fn oauth_autoconfig_auth_server(
    State(s): State<Arc<ServerState>>,
    _req: routes::oauth_autoconfig_auth_server::Request,
) -> Result<impl IntoResponse> {
    let config = AuthServerMetadata {
        issuer: s.config.api_url.clone(),
        authorization_response_iss_parameter_supported: true,
        registration_endpoint: registration_endpoint(&s)?,
        authorization_endpoint: s.config.html_url.join("/authorize")?,
        device_authorization_endpoint: s.config.api_url.join("/api/v1/oauth/device/start")?,
        token_endpoint: s.config.api_url.join("/api/v1/oauth/token")?,
        jwks_uri: s.config.api_url.join("/api/v1/.well-known/jwks.json")?,
        scopes_supported: Scope::iter().map(|a| a.to_string()).collect(),
        response_types_supported: vec![OauthResponseType::Code],
        grant_types_supported: vec![
            OauthGrantType::AuthorizationCode,
            OauthGrantType::ClientCredentials,
            OauthGrantType::DeviceCode,
        ],
        token_endpoint_auth_methods_supported: vec![
            OauthAuthMethod::ClientSecretPost,
            OauthAuthMethod::ClientSecretBasic,
            OauthAuthMethod::None,
        ],
        code_challenge_methods_supported: vec![
            OauthCodeChallengeMethod::S256,
            OauthCodeChallengeMethod::Plain,
        ],
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
        authorization_response_iss_parameter_supported: true,
        registration_endpoint: registration_endpoint(&s)?,
        authorization_endpoint: s.config.html_url.join("/authorize")?,
        device_authorization_endpoint: s.config.api_url.join("/api/v1/oauth/device/start")?,
        token_endpoint: s.config.api_url.join("/api/v1/oauth/token")?,
        userinfo_endpoint: s.config.api_url.join("/api/v1/oauth/userinfo")?,
        jwks_uri: s.config.api_url.join("/api/v1/.well-known/jwks.json")?,
        scopes_supported: Scope::iter().map(|a| a.to_string()).collect(),
        response_types_supported: vec![OauthResponseType::Code],
        grant_types_supported: vec![
            OauthGrantType::AuthorizationCode,
            OauthGrantType::ClientCredentials,
            OauthGrantType::DeviceCode,
        ],
        subject_types_supported: vec!["public".to_string()],
        id_token_signing_alg_values_supported: vec!["ES256".to_string()],
        token_endpoint_auth_methods_supported: vec![
            OauthAuthMethod::ClientSecretPost,
            OauthAuthMethod::ClientSecretBasic,
            OauthAuthMethod::None,
        ],
        claims_supported: vec![
            "iss".to_string(),
            "sub".to_string(),
            "auth_time".to_string(),
            "name".to_string(),
            "email".to_string(),
            "email_verified".to_string(),
            "picture".to_string(),
            "updated_at".to_string(),
            "profile".to_string(),
        ],
        code_challenge_methods_supported: vec![
            OauthCodeChallengeMethod::S256,
            OauthCodeChallengeMethod::Plain,
        ],
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

    // TODO: use MediaPaths for picture, some other path calculator for profile?
    let config = srv.state.config();
    let info = Userinfo {
        iss: config.api_url.clone(),
        sub: user.id,
        email: email.clone().map(|e| e.email),
        email_verified: email.map(|e| e.is_verified).unwrap_or_default(),
        name: user.name,
        profile: format!("{}user/{}", config.html_url, user.id),
        updated_at: user.version_id.get_timestamp().unwrap().to_unix().0,
        picture: user
            .avatar
            .map(|a| format!("{}media/{}", config.cdn_url, a))
            .and_then(|u| u.parse().ok()),
    };
    Ok(Json(info))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(oauth_info))
        .routes(routes2!(oauth_authorize))
        .routes(routes2!(oauth_register))
        .routes(routes2!(oauth_device_authorization))
        .routes(routes2!(oauth_device_info))
        .routes(routes2!(oauth_device_authorize))
        .routes(routes2!(oauth_token))
        .routes(routes2!(oauth_introspect))
        .routes(routes2!(oauth_revoke))
        .routes(routes2!(oauth_userinfo))
        .routes(routes2!(oauth_autoconfig_auth_server))
        .routes(routes2!(oauth_autoconfig_openid))
        .routes(routes2!(oauth_jwks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::v1::types::oauth::OauthClientRegistrationRequest;

    fn registration(redirect_uris: Vec<&str>) -> OauthClientRegistrationRequest {
        OauthClientRegistrationRequest {
            redirect_uris: redirect_uris.into_iter().map(ToString::to_string).collect(),
            token_endpoint_auth_method: None,
            grant_types: None,
            response_types: None,
            client_name: None,
            scope: None,
        }
    }

    #[test]
    fn redirect_uris_allow_https_and_loopback_http() {
        assert!(
            validate_redirect_uris(&[
                "https://example.com/callback".to_string(),
                "http://localhost:8080/callback".to_string(),
                "com.example.app:/callback".to_string(),
            ])
            .is_ok()
        );
    }

    #[test]
    fn redirect_uris_reject_fragments_and_remote_http() {
        assert!(
            validate_redirect_uris(&["https://example.com/callback#fragment".to_string()]).is_err()
        );
        assert!(validate_redirect_uris(&["http://example.com/callback".to_string()]).is_err());
    }

    #[test]
    fn registration_defaults_to_public_authorization_code() {
        let normalized =
            normalize_registration(registration(vec!["https://example.com/callback"])).unwrap();
        assert_eq!(normalized.client_name, "OAuth Client");
        assert_eq!(
            normalized.grant_types,
            vec![OauthGrantType::AuthorizationCode]
        );
        assert_eq!(normalized.response_types, vec![OauthResponseType::Code]);
    }

    #[test]
    fn registration_requires_redirect_uri_for_authorization_code() {
        assert!(normalize_registration(registration(vec![])).is_err());
    }

    #[test]
    fn registration_rejects_confidential_auth_method() {
        let mut request = registration(vec!["https://example.com/callback"]);
        request.token_endpoint_auth_method = Some(OauthAuthMethod::ClientSecretPost);
        assert!(normalize_registration(request).is_err());
    }

    #[test]
    fn redirect_uris_allow_ipv6_loopback() {
        assert!(validate_redirect_uris(&["http://[::1]:8080/callback".to_string()]).is_ok());
    }
}
