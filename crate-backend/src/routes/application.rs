use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Form, Json,
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use common::v1::types::{
    application::{Application, ApplicationCreate, ApplicationPatch, Scope},
    oauth::{
        Autoconfig, OauthAuthorizeInfo, OauthAuthorizeParams, OauthAuthorizeResponse,
        OauthIntrospectResponse, OauthTokenRequest, OauthTokenResponse, Userinfo,
    },
    util::{Changes, Diff, Time},
    ApplicationId, AuditLogChange, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Bot,
    BotAccess, ExternalPlatform, MessageSync, PaginationQuery, PaginationResponse, Permission,
    Puppet, PuppetCreate, RoomId, RoomMemberOrigin, RoomMemberPut, SessionCreate, SessionStatus,
    SessionToken, SessionType, SessionWithToken, User, UserId,
};
use headers::HeaderMapExt;
use http::{HeaderMap, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::{
    routes::util::{AuthWithSession, HeaderReason},
    types::{DbSessionCreate, DbUserCreate},
    ServerState,
};

use super::util::Auth;
use crate::error::{Error, Result};

/// App create
#[utoipa::path(
    post,
    path = "/app",
    tags = ["application"],
    request_body = ApplicationCreate,
    responses(
        (status = CREATED, description = "success", body = Application)
    )
)]
async fn app_create(
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ApplicationCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id: Some(auth_user.id),
            name: json.name.clone(),
            description: json.description.clone(),
            bot: Some(Bot {
                owner_id: auth_user.id,
                access: if json.public {
                    return Err(Error::Unimplemented);
                } else {
                    BotAccess::Private
                },
                is_bridge: json.bridge,
            }),
            puppet: None,
            registered_at: Some(Time::now_utc()),
            system: false,
        })
        .await?;
    let app = Application {
        id: user.id.into_inner().into(),
        owner_id: auth_user.id,
        name: json.name,
        description: json.description,
        bridge: json.bridge,
        public: json.public,
        oauth_secret: None,
        oauth_redirect_uris: vec![],
        oauth_confidential: false,
    };
    data.application_insert(app.clone()).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::ApplicationCreate {
            application_id: app.id,
            changes: Changes::new()
                .add("name", &app.name)
                .add("description", &app.description)
                .add("bridge", &app.bridge)
                .add("public", &app.public)
                .build(),
        },
    })
    .await?;
    Ok((StatusCode::CREATED, Json(app)))
}

/// App list
#[utoipa::path(
    get,
    path = "/app",
    tags = ["application"],
    params(PaginationQuery<ApplicationId>),
    responses(
        (status = OK, description = "success", body = PaginationResponse<Application>)
    )
)]
async fn app_list(
    Auth(auth_user): Auth,
    Query(q): Query<PaginationQuery<ApplicationId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut list = data.application_list(auth_user.id, q).await?;
    for app in &mut list.items {
        app.oauth_secret = None;
    }
    Ok(Json(list))
}

/// App get
#[utoipa::path(
    get,
    path = "/app/{app_id}",
    tags = ["application"],
    responses(
        (status = OK, description = "success", body = Application)
    )
)]
async fn app_get(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    app.oauth_secret = None;
    if app.owner_id == auth_user.id || app.public {
        Ok(Json(app))
    } else {
        Err(Error::NotFound)
    }
}

/// App patch
#[utoipa::path(
    patch,
    path = "/app/{app_id}",
    tags = ["application"],
    request_body = ApplicationPatch,
    responses(
        (status = OK, description = "success", body = Application)
    )
)]
async fn app_patch(
    Path((app_id,)): Path<(ApplicationId,)>,
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(patch): Json<ApplicationPatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    patch.validate()?;
    let data = s.data();
    let start = data.application_get(app_id).await?;
    if start.owner_id != auth_user.id {
        return Err(Error::MissingPermissions);
    }

    if !patch.changes(&start) {
        return Err(Error::NotModified);
    }

    let mut app = start.clone();
    app.name = patch.name.unwrap_or(app.name);
    app.description = patch.description.unwrap_or(app.description);
    app.bridge = patch.bridge.unwrap_or(app.bridge);
    app.public = patch.public.unwrap_or(app.public);
    app.oauth_redirect_uris = patch.oauth_redirect_uris.unwrap_or(app.oauth_redirect_uris);
    app.oauth_confidential = patch.oauth_confidential.unwrap_or(app.oauth_confidential);

    data.application_update(app.clone()).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::ApplicationUpdate {
            application_id: app.id,
            changes: Changes::new()
                .change("name", &start.name, &app.name)
                .change("description", &start.description, &app.description)
                .change("bridge", &start.bridge, &app.bridge)
                .change("public", &start.public, &app.public)
                .change(
                    "oauth_redirect_uris",
                    &start.oauth_redirect_uris,
                    &app.oauth_redirect_uris,
                )
                .change(
                    "oauth_confidential",
                    &start.oauth_confidential,
                    &app.oauth_confidential,
                )
                .build(),
        },
    })
    .await?;

    Ok(Json(app))
}

/// App delete
#[utoipa::path(
    delete,
    path = "/app/{app_id}",
    tags = ["application"],
    responses(
        (status = NO_CONTENT, description = "success")
    )
)]
async fn app_delete(
    Path((app_id,)): Path<(ApplicationId,)>,
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth_user.id {
        data.application_delete(app_id).await?;
        data.user_delete(app_id.into_inner().into()).await?;
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: auth_user.id.into_inner().into(),
            user_id: auth_user.id,
            session_id: Some(session.id),
            reason,
            ty: AuditLogEntryType::ApplicationDelete {
                application_id: app_id,
            },
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// App create session
#[utoipa::path(
    post,
    path = "/app/{app_id}/session",
    tags = ["application"],
    request_body = SessionCreate,
    responses(
        (status = CREATED, description = "success", body = SessionWithToken)
    )
)]
async fn app_create_session(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<SessionCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth_user.id {
        let token = SessionToken(Uuid::new_v4().to_string()); // TODO: is this secure enough
        let session = data
            .session_create(DbSessionCreate {
                token: token.clone(),
                name: json.name,
                expires_at: None,
                ty: SessionType::User,
                application_id: None,
            })
            .await?;
        data.session_set_status(
            session.id,
            SessionStatus::Authorized {
                user_id: app.id.into_inner().into(),
            },
        )
        .await?;
        let session = data.session_get(session.id).await?;
        let session_with_token = SessionWithToken { session, token };
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: auth_user.id.into_inner().into(),
            user_id: auth_user.id,
            session_id: Some(session_with_token.session.id),
            reason,
            ty: AuditLogEntryType::SessionLogin {
                user_id: app.id.into_inner().into(),
                session_id: session_with_token.session.id,
            },
        })
        .await?;
        Ok((StatusCode::CREATED, Json(session_with_token)))
    } else {
        Err(Error::MissingPermissions)
    }
}

#[derive(Deserialize, ToSchema)]
struct AppInviteBot {
    room_id: RoomId,
}

/// App invite bot
///
/// Add a bot to a room
#[utoipa::path(
    post,
    path = "/app/{app_id}/invite",
    tags = ["application", "badge.perm.BotsAdd"],
    request_body = AppInviteBot,
    responses(
        (status = NO_CONTENT, description = "success")
    )
)]
async fn app_invite_bot(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AppInviteBot>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;

    if !app.public && app.owner_id != auth_user.id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    let perms = srv.perms.for_room(auth_user.id, json.room_id).await?;
    perms.ensure(Permission::BotsAdd)?;

    let bot_user_id: UserId = app.id.into_inner().into();

    if data.room_ban_get(json.room_id, bot_user_id).await.is_ok() {
        return Err(Error::BadStatic("banned"));
    }

    let origin = RoomMemberOrigin::BotInstall {
        user_id: auth_user.id,
    };
    data.room_member_put(
        json.room_id,
        bot_user_id,
        Some(origin),
        RoomMemberPut::default(),
    )
    .await?;

    let member = data.room_member_get(json.room_id, bot_user_id).await?;

    s.broadcast_room(
        json.room_id,
        auth_user.id,
        MessageSync::RoomMemberUpsert {
            member: member.clone(),
        },
    )
    .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: json.room_id,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::BotAdd {
            bot_id: bot_user_id,
        },
    })
    .await?;

    srv.rooms
        .send_welcome_message(json.room_id, bot_user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Puppet ensure
#[utoipa::path(
    put,
    path = "/app/{app_id}/puppet/{puppet_id}",
    tags = ["application"],
    request_body = PuppetCreate,
    responses(
        (status = OK, description = "success", body = User),
        (status = CREATED, description = "created", body = User)
    )
)]
async fn puppet_ensure(
    Path((app_id, puppet_id)): Path<(ApplicationId, String)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PuppetCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    if *app_id != *auth_user.id {
        return Err(Error::MissingPermissions);
    }

    let parent_id = Some(auth_user.id);
    let data = s.data();
    let srv = s.services();
    let parent = srv.users.get(auth_user.id).await?;
    if !parent.bot.is_some_and(|b| b.is_bridge) {
        return Err(Error::BadStatic("can't create that user"));
    };
    let existing = data.user_lookup_puppet(auth_user.id, &puppet_id).await?;
    if let Some(id) = existing {
        let user = data.user_get(id).await?;
        return Ok((StatusCode::OK, Json(user)));
    }
    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id,
            name: json.name,
            description: json.description,
            bot: None,
            puppet: Some(Puppet {
                owner_id: auth_user.id,
                external_platform: ExternalPlatform::Discord,
                external_id: puppet_id.clone(),
                external_url: None,
                alias_id: None,
            }),
            registered_at: Some(Time::now_utc()),
            system: false,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(user)))
}

/// App rotate oauth secret
#[utoipa::path(
    post,
    path = "/app/{app_id}/rotate-secret",
    tags = ["application"],
    responses(
        (status = OK, description = "success", body = Application)
    )
)]
async fn app_rotate_secret(
    Path((app_id,)): Path<(ApplicationId,)>,
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    if app.owner_id != auth_user.id {
        return Err(Error::MissingPermissions);
    }
    app.oauth_secret = Some(Uuid::new_v4().to_string());
    data.application_update(app.clone()).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::ApplicationUpdate {
            application_id: app.id,
            changes: vec![AuditLogChange {
                key: "oauth_secret".into(),
                old: Value::Null,
                new: Value::Null,
            }],
        },
    })
    .await?;
    Ok(Json(app))
}

/// Oauth info
#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tags = ["application"],
    params(OauthAuthorizeParams),
    responses(
        (status = OK, description = "success", body = OauthAuthorizeInfo)
    )
)]
async fn oauth_info(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<OauthAuthorizeParams>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let app = data.application_get(q.client_id).await?;
    if app.owner_id != auth_user.id && !app.public {
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
    let auth_user = srv.users.get(auth_user.id).await?;
    let bot_user = srv.users.get(app.id.into_inner().into()).await?;
    let authorized = if let Ok(existing) = data.connection_get(auth_user.id, app.id).await {
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
    tags = ["application"],
    params(OauthAuthorizeParams),
    responses(
        (status = OK, description = "success", body = OauthAuthorizeResponse)
    )
)]
async fn oauth_authorize(
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<OauthAuthorizeParams>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let app = data.application_get(q.client_id).await?;
    if app.owner_id != auth_user.id && !app.public {
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
    let scopes: Vec<_> = scopes.into_iter().collect();
    data.connection_create(auth_user.id, app.id, scopes.clone())
        .await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
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
        auth_user.id,
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
    tags = ["application"],
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
    tags = ["application"],
    responses(
        (status = OK, description = "success", body = OauthIntrospectResponse)
    )
)]
async fn oauth_introspect(
    AuthWithSession(session, user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let Some(app_id) = session.app_id else {
        return Err(Error::BadStatic("not an oauth token"));
    };
    let connection = s.data().connection_get(user.id, app_id).await?;
    let res = OauthIntrospectResponse {
        active: true,
        scopes: connection.scopes,
        client_id: app_id,
        username: user.id,
        exp: session.expires_at.map(|e| e.unix_timestamp() as u64),
    };
    Ok(Json(res))
}

/// Oauth revoke
#[utoipa::path(
    post,
    path = "/oauth/revoke",
    tags = ["application"],
    responses(
        (status = NO_CONTENT, description = "success")
    )
)]
async fn oauth_revoke(
    AuthWithSession(session, _): AuthWithSession,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    data.session_delete(session.id).await?;
    srv.sessions.invalidate(session.id).await;
    Ok(())
}

/// Oauth autoconfig
#[utoipa::path(
    get,
    path = "/oauth/.well-known/openid-configuration",
    tags = ["application"],
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
    tags = ["application"],
    responses(
        (status = OK, description = "success", body = Userinfo)
    )
)]
async fn oauth_userinfo(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let user = srv.users.get(auth_user.id).await?;
    let email = data
        .user_email_list(auth_user.id)
        .await?
        .into_iter()
        .find(|e| e.is_primary);
    let info = Userinfo {
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
        .routes(routes!(app_create))
        .routes(routes!(app_list))
        .routes(routes!(app_get))
        .routes(routes!(app_patch))
        .routes(routes!(app_delete))
        .routes(routes!(app_create_session))
        .routes(routes!(puppet_ensure))
        .routes(routes!(app_invite_bot))
        .routes(routes!(app_rotate_secret))
        .routes(routes!(oauth_info))
        .routes(routes!(oauth_authorize))
        .routes(routes!(oauth_token))
        .routes(routes!(oauth_introspect))
        .routes(routes!(oauth_revoke))
        .routes(routes!(oauth_userinfo))
        .routes(routes!(oauth_autoconfig))
}
