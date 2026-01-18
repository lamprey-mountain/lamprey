use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    application::{Application, ApplicationCreate, ApplicationPatch},
    misc::ApplicationIdReq,
    util::{Changes, Diff, Time},
    ApplicationId, AuditLogChange, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync,
    PaginationQuery, PaginationResponse, Permission, Puppet, PuppetCreate, RoomId,
    RoomMemberOrigin, RoomMemberPut, SessionCreate, SessionStatus, SessionToken, SessionType,
    SessionWithToken, User, UserId,
};
use http::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::{
    routes::util::{Auth, HeaderReason},
    types::{DbSessionCreate, DbUserCreate},
    ServerState,
};

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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<ApplicationCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    if let Some(bridge) = &json.bridge {
        if bridge.platform_name.is_none() {
            return Err(Error::BadStatic("platform_name is required for bridge"));
        }
    }

    let data = s.data();
    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id: Some(auth.user.id),
            name: json.name.clone(),
            description: json.description.clone(),
            puppet: None,
            registered_at: Some(Time::now_utc()),
            system: false,
        })
        .await?;
    let app = Application {
        id: user.id.into_inner().into(),
        owner_id: auth.user.id,
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
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
    auth: Auth,
    Query(q): Query<PaginationQuery<ApplicationId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut list = data.application_list(auth.user.id, q).await?;
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
    Path((app_id,)): Path<(ApplicationIdReq,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let app_id = match app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    app.oauth_secret = None;
    if app.owner_id == auth.user.id || app.public || *app.id == *auth.user.id {
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
    Path((app_id,)): Path<(ApplicationIdReq,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(patch): Json<ApplicationPatch>,
) -> Result<impl IntoResponse> {
    let app_id = match app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    auth.user.ensure_unsuspended()?;
    patch.validate()?;
    let data = s.data();
    let start = data.application_get(app_id).await?;
    if start.owner_id != auth.user.id && *start.id != *auth.user.id {
        return Err(Error::MissingPermissions);
    }

    if let Some(Some(bridge)) = &patch.bridge {
        if bridge.platform_name.is_none() {
            return Err(Error::BadStatic("platform_name is required for bridge"));
        }
    }

    if !patch.changes(&start) {
        return Ok(Json(start));
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
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth.user.id {
        data.application_delete(app_id).await?;
        data.user_delete(app_id.into_inner().into()).await?;
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: auth.user.id.into_inner().into(),
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::ApplicationDelete {
                application_id: app_id,
                changes: Changes::new()
                    .remove("name", &app.name)
                    .remove("description", &app.description)
                    .remove("bridge", &app.bridge)
                    .remove("public", &app.public)
                    .build(),
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
    Path((app_id,)): Path<(ApplicationIdReq,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<SessionCreate>,
) -> Result<impl IntoResponse> {
    let app_id = match app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;
    if app.owner_id == auth.user.id || *app.id == *auth.user.id {
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
            room_id: auth.user.id.into_inner().into(),
            user_id: auth.user.id,
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AppInviteBot>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let app = data.application_get(app_id).await?;

    if !app.public && app.owner_id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, json.room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let bot_user_id: UserId = app.id.into_inner().into();

    if data.room_ban_get(json.room_id, bot_user_id).await.is_ok() {
        return Err(Error::BadStatic("banned"));
    }

    let origin = RoomMemberOrigin::BotInstall {
        user_id: auth.user.id,
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
        auth.user.id,
        MessageSync::RoomMemberUpsert {
            member: member.clone(),
        },
    )
    .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: json.room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
    Path((app_id, puppet_id)): Path<(ApplicationIdReq, String)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PuppetCreate>,
) -> Result<impl IntoResponse> {
    let app_id = match app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    auth.user.ensure_unsuspended()?;

    if *app_id != *auth.user.id {
        return Err(Error::MissingPermissions);
    }

    let parent_id = Some(auth.user.id);
    let data = s.data();
    let srv = s.services();
    let parent = srv.users.get(auth.user.id, None).await?;
    if !parent.bot {
        // TODO: check if it is a bridge?
        return Err(Error::BadStatic("can't create that user"));
    };
    let existing = data.user_lookup_puppet(auth.user.id, &puppet_id).await?;
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
            puppet: Some(Puppet {
                owner_id: auth.user.id.into_inner().into(), // ApplicationId
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
    Path((app_id,)): Path<(ApplicationIdReq,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let app_id = match app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    if app.owner_id != auth.user.id && *app.id != *auth.user.id {
        return Err(Error::MissingPermissions);
    }
    app.oauth_secret = Some(Uuid::new_v4().to_string());
    data.application_update(app.clone()).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
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
}
