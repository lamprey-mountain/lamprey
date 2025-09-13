use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    application::{Application, ApplicationCreate, ApplicationPatch, Scope},
    email::EmailAddr,
    util::{Diff, Time},
    ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Bot, BotAccess,
    ExternalPlatform, MessageSync, PaginationQuery, Permission, Puppet, PuppetCreate, RoomId,
    RoomMemberOrigin, RoomMemberPut, SessionCreate, SessionStatus, SessionToken, SessionWithToken,
    UserId,
};
use http::StatusCode;
use serde::Deserialize;
use url::Url;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::{routes::util::HeaderReason, types::DbUserCreate, ServerState};

use super::util::Auth;
use crate::error::{Error, Result};

/// App create
#[utoipa::path(
    post,
    path = "/app",
    tags = ["application"],
    responses((status = CREATED, description = "success"))
)]
async fn app_create(
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<ApplicationCreate>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let user = data
        .user_create(DbUserCreate {
            parent_id: Some(auth_user_id),
            name: json.name.clone(),
            description: json.description.clone(),
            bot: Some(Bot {
                owner_id: auth_user_id,
                access: if json.public {
                    return Err(Error::Unimplemented);
                } else {
                    BotAccess::Private
                },
                is_bridge: json.bridge,
            }),
            puppet: None,
            registered_at: Some(Time::now_utc()),
        })
        .await?;
    let app = Application {
        id: user.id.into_inner().into(),
        owner_id: auth_user_id,
        name: json.name,
        description: json.description,
        bridge: json.bridge,
        public: json.public,
        oauth_secret: None,
        oauth_redirect_uris: vec![],
        oauth_confidential: false,
    };
    data.application_insert(app.clone()).await?;
    Ok(Json(app))
}

/// App list
#[utoipa::path(
    get,
    path = "/app",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_list(
    Auth(auth_user_id): Auth,
    Query(q): Query<PaginationQuery<ApplicationId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let list = data.application_list(auth_user_id, q).await?;
    Ok(Json(list))
}

/// App get
#[utoipa::path(
    get,
    path = "/app/{app_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_get(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth_user_id {
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
    responses((status = OK, description = "success"))
)]
async fn app_patch(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<ApplicationPatch>,
) -> Result<impl IntoResponse> {
    patch.validate()?;
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    if app.owner_id != auth_user_id {
        return Err(Error::MissingPermissions);
    }

    if !patch.changes(&app) {
        return Err(Error::NotModified);
    }

    app.name = patch.name.unwrap_or(app.name);
    app.description = patch.description.unwrap_or(app.description);
    app.bridge = patch.bridge.unwrap_or(app.bridge);
    app.public = patch.public.unwrap_or(app.public);
    app.oauth_redirect_uris = patch.oauth_redirect_uris.unwrap_or(app.oauth_redirect_uris);
    app.oauth_confidential = patch.oauth_confidential.unwrap_or(app.oauth_confidential);

    data.application_update(app.clone()).await?;

    Ok(Json(app))
}

/// App delete
#[utoipa::path(
    delete,
    path = "/app/{app_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_delete(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth_user_id {
        data.application_delete(app_id).await?;
        data.user_delete(app_id.into_inner().into()).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// App create session
#[utoipa::path(
    post,
    path = "/app/{app_id}/session",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_create_session(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<SessionCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth_user_id {
        let token = SessionToken(Uuid::new_v4().to_string()); // TODO: is this secure enough
        let session = data.session_create(token.clone(), json.name).await?;
        data.session_set_status(
            session.id,
            SessionStatus::Authorized {
                user_id: app.id.into_inner().into(),
            },
        )
        .await?;
        let session = data.session_get(session.id).await?;
        let session_with_token = SessionWithToken { session, token };
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
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_invite_bot(
    Path((app_id,)): Path<(ApplicationId,)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AppInviteBot>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let app = data.application_get(app_id).await?;

    if !app.public && app.owner_id != auth_user_id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    let perms = srv.perms.for_room(auth_user_id, json.room_id).await?;
    perms.ensure(Permission::BotsAdd)?;

    let bot_user_id: UserId = app.id.into_inner().into();

    if data.room_ban_get(json.room_id, bot_user_id).await.is_ok() {
        return Err(Error::BadStatic("banned"));
    }

    let origin = RoomMemberOrigin::BotInstall {
        user_id: auth_user_id,
    };
    data.room_member_put(json.room_id, bot_user_id, origin, RoomMemberPut::default())
        .await?;

    let member = data.room_member_get(json.room_id, bot_user_id).await?;

    s.broadcast_room(
        json.room_id,
        auth_user_id,
        MessageSync::RoomMemberUpsert {
            member: member.clone(),
        },
    )
    .await?;

    data.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: json.room_id,
        user_id: auth_user_id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::BotAdd {
            bot_id: bot_user_id,
        },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Puppet ensure
#[utoipa::path(
    put,
    path = "/app/{app_id}/puppet/{puppet_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn puppet_ensure(
    Path((app_id, puppet_id)): Path<(ApplicationId, String)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PuppetCreate>,
) -> Result<impl IntoResponse> {
    if *app_id != *auth_user_id {
        return Err(Error::MissingPermissions);
    }

    let parent_id = Some(auth_user_id);
    let data = s.data();
    let srv = s.services();
    let parent = srv.users.get(auth_user_id).await?;
    if !parent.bot.is_some_and(|b| b.is_bridge) {
        return Err(Error::BadStatic("can't create that user"));
    };
    let existing = data.user_lookup_puppet(auth_user_id, &puppet_id).await?;
    if let Some(id) = existing {
        let user = data.user_get(id).await?;
        return Ok((StatusCode::OK, Json(user)));
    }
    let user = data
        .user_create(DbUserCreate {
            parent_id,
            name: json.name,
            description: json.description,
            bot: None,
            puppet: Some(Puppet {
                owner_id: auth_user_id,
                external_platform: ExternalPlatform::Discord,
                external_id: puppet_id.clone(),
                external_url: None,
                alias_id: None,
            }),
            registered_at: Some(Time::now_utc()),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(user)))
}

/// App rotate oauth secret (TODO)
#[utoipa::path(
    post,
    path = "/app/{app_id}/rotate-secret",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_rotate_secret(
    Path((_app_id,)): Path<(ApplicationId,)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(())
}

/// Oauth exchange token (TODO)
///
/// exchange an authorization token for an access token
#[utoipa::path(
    post,
    path = "/oauth/token",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn oauth_token() -> Result<impl IntoResponse> {
    Ok(())
}

/// Oauth introspect (TODO)
#[utoipa::path(
    post,
    path = "/oauth/introspect",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn oauth_introspect() -> Result<impl IntoResponse> {
    Ok(())
}

/// Oauth revoke (TODO)
#[utoipa::path(
    post,
    path = "/oauth/revoke",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn oauth_revoke() -> Result<impl IntoResponse> {
    Ok(())
}

/// Oauth autoconfig (TODO)
#[utoipa::path(
    get,
    path = "/oauth/.well-known/openid-configuration",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn oauth_autoconfig() -> Result<impl IntoResponse> {
    Ok(())
}

/// Oauth autoconfig (TODO)
#[utoipa::path(
    get,
    path = "/oauth/userinfo",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn oauth_userinfo() -> Result<impl IntoResponse> {
    Ok(())
}

/// openid connect automatic configuration
struct Autoconfig {
    issuer: Url,
    /// this uses the html endpoint
    // https://example.com/authorize?stuff=goes&here
    authorization_endpoint: Url,
    token_endpoint: Url,
    userinfo_endpoint: Url,

    /// code | token | id_token
    response_types_supported: Vec<String>,

    /// authorization_code | refresh_token
    grant_types_supported: Vec<String>,
}

struct Userinfo {
    sub: UserId,

    /// primary email address (is None if email scope isnt provided)
    email: Option<EmailAddr>,
    email_verified: bool,

    name: String,

    // https://example.com/user/{user_id}
    // where example.com is html_url and user_id is the user's uuid
    profile: String,

    /// calculated from version_id
    updated_at: u64,

    /// link to https://cdn.example.com/media/{media_id}
    picture: Option<Url>,
}

struct IntrospectResponse {
    active: bool,
    scopes: Vec<Scope>,
    client_id: ApplicationId,
    /// this is specified to be "human readable", but in practice it would be
    /// simpler and more useful to return the unique id of the user
    username: UserId,
    exp: Option<u64>,
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
        .routes(routes!(oauth_token))
        .routes(routes!(oauth_introspect))
        .routes(routes!(oauth_revoke))
        .routes(routes!(oauth_userinfo))
        .routes(routes!(oauth_revoke))
        .routes(routes!(oauth_autoconfig))
}
