use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use common::v1::routes;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::misc::ApplicationIdReq;
use common::v1::types::{
    application::Application,
    util::{Changes, Diff, Time},
    AuditLogChange, AuditLogEntryType, MessageSync, Permission, Puppet, RoomMemberOrigin,
    RoomMemberPut, SessionStatus, SessionToken, SessionType, SessionWithToken, UserId,
};
use http::StatusCode;
use lamprey_macros::handler;
use serde_json::Value;
use uuid::Uuid;
use validator::Validate;

use crate::{
    routes::util::Auth,
    types::{DbSessionCreate, DbUserCreate},
    ServerState,
};
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes2;

/// App create
#[handler(routes::app_create)]
async fn app_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_create::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    srv.perms
        .for_server(auth.user.id)
        .await?
        .ensure(Permission::ApplicationCreate)?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    let json = req.application;
    if let Some(bridge) = &json.bridge {
        if bridge.platform_name.is_none() {
            return Err(ApiError::from_code(ErrorCode::PlatformNameRequiredForBridge).into());
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
    al.commit_success(AuditLogEntryType::ApplicationCreate {
        application_id: app.id,
        changes: Changes::new()
            .add("name", &app.name)
            .add("description", &app.description)
            .add("bridge", &app.bridge)
            .add("public", &app.public)
            .build(),
    })
    .await?;
    Ok((StatusCode::CREATED, Json(app)))
}

/// App list
#[handler(routes::app_list)]
async fn app_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_list::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut list = data.application_list(auth.user.id, req.pagination).await?;
    for app in &mut list.items {
        app.oauth_secret = None;
    }
    Ok(Json(list))
}

/// App get
#[handler(routes::app_get)]
async fn app_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_get::Request,
) -> Result<impl IntoResponse> {
    let app_id = match req.app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    app.oauth_secret = None;
    if app.owner_id == auth.user.id || app.public || *app.id == *auth.user.id {
        Ok(Json(app))
    } else {
        Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownApplication,
        )))
    }
}

/// App patch
#[handler(routes::app_patch)]
async fn app_patch(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_patch::Request,
) -> Result<impl IntoResponse> {
    let app_id = match req.app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    auth.user.ensure_unsuspended()?;
    let al = auth.audit_log(auth.user.id.into_inner().into());
    let patch = req.patch;
    patch.validate()?;
    let data = s.data();
    let start = data.application_get(app_id).await?;
    if start.owner_id != auth.user.id && *start.id != *auth.user.id {
        return Err(Error::MissingPermissions);
    }

    if let Some(Some(bridge)) = &patch.bridge {
        if bridge.platform_name.is_none() {
            return Err(ApiError::from_code(ErrorCode::PlatformNameRequiredForBridge).into());
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

    al.commit_success(AuditLogEntryType::ApplicationUpdate {
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
    })
    .await?;

    Ok(Json(app))
}

/// App delete
#[handler(routes::app_delete)]
async fn app_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let al = auth.audit_log(auth.user.id.into_inner().into());
    let data = s.data();
    let app = data.application_get(req.app_id).await?;
    if app.owner_id == auth.user.id {
        data.application_delete(req.app_id).await?;
        data.user_delete(req.app_id.into_inner().into()).await?;
        al.commit_success(AuditLogEntryType::ApplicationDelete {
            application_id: req.app_id,
            changes: Changes::new()
                .remove("name", &app.name)
                .remove("description", &app.description)
                .remove("bridge", &app.bridge)
                .remove("public", &app.public)
                .build(),
        })
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// App create session
#[handler(routes::app_create_session)]
async fn app_create_session(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_create_session::Request,
) -> Result<impl IntoResponse> {
    let app_id = match req.app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    auth.user.ensure_unsuspended()?;
    let al = auth.audit_log(auth.user.id.into_inner().into());
    let json = req.session;
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
                ip_addr: auth.session.ip_addr.clone(),
                user_agent: auth.session.user_agent.clone(),
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
        al.commit_success(AuditLogEntryType::SessionLogin {
            user_id: app.id.into_inner().into(),
            session_id: session_with_token.session.id,
        })
        .await?;
        Ok((StatusCode::CREATED, Json(session_with_token)))
    } else {
        Err(Error::MissingPermissions)
    }
}

/// App invite bot
///
/// Add a bot to a room
#[handler(routes::app_invite_bot)]
async fn app_invite_bot(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_invite_bot::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let app = data.application_get(req.app_id).await?;

    if !app.public && app.owner_id != auth.user.id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let bot_user_id: UserId = app.id.into_inner().into();

    if data.room_ban_get(req.room_id, bot_user_id).await.is_ok() {
        return Err(ApiError::from_code(ErrorCode::YouAreBanned).into());
    }

    let origin = RoomMemberOrigin::BotInstall {
        user_id: auth.user.id,
    };
    data.room_member_put(
        req.room_id,
        bot_user_id,
        Some(origin),
        RoomMemberPut::default(),
    )
    .await?;

    let member = data.room_member_get(req.room_id, bot_user_id).await?;
    let user = srv.users.get(bot_user_id, None).await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::RoomMemberCreate {
            member: member.clone(),
            user,
        },
    )
    .await?;

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::BotAdd {
        bot_id: bot_user_id,
    })
    .await?;

    srv.rooms
        .send_welcome_message(req.room_id, bot_user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Puppet ensure
#[handler(routes::puppet_ensure)]
async fn puppet_ensure(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::puppet_ensure::Request,
) -> Result<impl IntoResponse> {
    let app_id = match req.app_id {
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
        return Err(ApiError::from_code(ErrorCode::CantCreateThatUser).into());
    };
    let existing = data
        .user_lookup_puppet(auth.user.id, &req.puppet_id)
        .await?;
    if let Some(id) = existing {
        let user = data.user_get(id).await?;
        return Ok((StatusCode::OK, Json(user)));
    }
    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id,
            name: req.puppet.name,
            description: req.puppet.description,
            puppet: Some(Puppet {
                owner_id: auth.user.id.into_inner().into(), // ApplicationId
                external_id: req.puppet_id.clone(),
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
#[handler(routes::app_rotate_secret)]
async fn app_rotate_secret(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::app_rotate_secret::Request,
) -> Result<impl IntoResponse> {
    let app_id = match req.app_id {
        ApplicationIdReq::AppSelf => (*auth.user.id).into(),
        ApplicationIdReq::ApplicationId(id) => id,
    };
    let al = auth.audit_log(auth.user.id.into_inner().into());
    let data = s.data();
    let mut app = data.application_get(app_id).await?;
    if app.owner_id != auth.user.id && *app.id != *auth.user.id {
        return Err(Error::MissingPermissions);
    }
    app.oauth_secret = Some(Uuid::new_v4().to_string());
    data.application_update(app.clone()).await?;
    al.commit_success(AuditLogEntryType::ApplicationUpdate {
        application_id: app.id,
        changes: vec![AuditLogChange {
            key: "oauth_secret".into(),
            old: Value::Null,
            new: Value::Null,
        }],
    })
    .await?;
    Ok(Json(app))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(app_create))
        .routes(routes2!(app_list))
        .routes(routes2!(app_get))
        .routes(routes2!(app_patch))
        .routes(routes2!(app_delete))
        .routes(routes2!(app_create_session))
        .routes(routes2!(puppet_ensure))
        .routes(routes2!(app_invite_bot))
        .routes(routes2!(app_rotate_secret))
}
