use std::sync::Arc;

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use common::v1::routes;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Time;
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryStatus, AuditLogEntryType, MessageSync,
    RoomMemberPut, SERVER_ROOM_ID, SessionStatus,
};
use lamprey_macros::handler;
use tracing::debug;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::config::ConfigOauthProviderKind;
use crate::error::{Error, Result};
use crate::routes::util::Auth3;
use crate::types::DbUserCreate;
use crate::{ServerState, routes2};

/// a normalized profile fetched from an oauth/oidc provider
struct OauthProfile {
    /// the provider-specific unique user id
    remote_id: String,
    /// the display name to use when creating a new user
    name: String,
    /// an optional description for a newly created user
    description: Option<String>,
}

/// Auth oauth redirect
#[handler(routes::auth_oauth_redirect)]
async fn auth_oauth_redirect(
    _auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::auth_oauth_redirect::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let mut data = s.data();
    let provider_config = s
        .config
        .oauth_provider
        .get(&req.provider)
        .ok_or(Error::Unimplemented)?;

    let state = Uuid::parse_str(&req.params.state)
        .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;

    let (oauth_token, session_id) = srv
        .oauth
        .exchange_code_for_token(state, req.params.code)
        .await?;

    let profile = match provider_config.kind {
        ConfigOauthProviderKind::Discord => {
            let u = srv.oauth.discord_get_user(oauth_token.access_token).await?;
            debug!("new discord user {:?}", u);
            OauthProfile {
                remote_id: u.user.id,
                name: u.user.global_name.unwrap_or(u.user.username),
                description: None,
            }
        }
        ConfigOauthProviderKind::Github => {
            let u = srv.oauth.github_get_user(oauth_token.access_token).await?;
            debug!("new github user {:?}", u);
            OauthProfile {
                remote_id: u.id.to_string(),
                name: u.name.unwrap_or(u.login),
                description: u.bio,
            }
        }
        ConfigOauthProviderKind::Oidc => {
            let u = srv
                .oauth
                .oidc_get_user(&req.provider, oauth_token.access_token)
                .await?;
            debug!("new oidc user {:?}", u);
            OauthProfile {
                remote_id: u.sub,
                name: u
                    .name
                    .or(u.preferred_username)
                    .unwrap_or_else(|| "unknown".to_string()),
                description: None,
            }
        }
    };

    let user_id = match data
        .auth_oauth_get_remote(req.provider.clone(), profile.remote_id.clone())
        .await?
    {
        Some(user_id) => {
            let user = srv.users.get(user_id, None).await?;
            data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                .await?;
            srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

            if provider_config.autoregister && user.registered_at.is_none() {
                data.user_set_registered(user.id, Some(Time::now_utc()), None)
                    .await?;
                srv.users.invalidate(user.id).await;
                let updated_user = srv.users.get(user.id, None).await?;
                let session = srv.sessions.get(session_id).await?;
                let entry = AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id: SERVER_ROOM_ID,
                    user_id: user.id,
                    session_id: Some(session_id),
                    reason: None,
                    ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                    status: AuditLogEntryStatus::Success,
                    started_at: Time::now_utc(),
                    ended_at: Time::now_utc(),
                    ip_addr: session.imprint.ip_addr.clone(),
                    user_agent: session.imprint.user_agent.clone(),
                    application_id: session.app_id,
                };
                data.audit_logs_room_append(entry.clone()).await?;
                s.broadcast_room(
                    entry.room_id,
                    entry.user_id,
                    MessageSync::AuditLogEntryCreate { entry },
                )
                .await?;
                s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
            }
            user_id
        }
        None => {
            let registered_at = if provider_config.autoregister {
                Some(Time::now_utc())
            } else {
                None
            };
            let user = data
                .user_create(DbUserCreate {
                    id: None,
                    parent_id: None,
                    name: profile.name,
                    description: profile.description,
                    puppet: None,
                    registered_at,
                    system: false,
                    remote: None,
                })
                .await?;
            data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                .await?;
            srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

            if provider_config.autoregister {
                let session = srv.sessions.get(session_id).await?;
                let entry = AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id: SERVER_ROOM_ID,
                    user_id: user.id,
                    session_id: Some(session_id),
                    reason: None,
                    ty: AuditLogEntryType::UserRegistered { user_id: user.id },
                    status: AuditLogEntryStatus::Success,
                    started_at: Time::now_utc(),
                    ended_at: Time::now_utc(),
                    ip_addr: session.imprint.ip_addr.clone(),
                    user_agent: session.imprint.user_agent.clone(),
                    application_id: session.app_id,
                };
                data.audit_logs_room_append(entry.clone()).await?;
                s.broadcast_room(
                    entry.room_id,
                    entry.user_id,
                    MessageSync::AuditLogEntryCreate { entry },
                )
                .await?;
            }
            data.auth_oauth_put(req.provider.clone(), user.id, profile.remote_id, true)
                .await?;
            user.id
        }
    };

    data.session_set_status(session_id, SessionStatus::Authorized { user_id })
        .await?;
    srv.sessions.invalidate(session_id).await;
    let session = srv.sessions.get(session_id).await?;
    s.broadcast(MessageSync::SessionCreate {
        session: (*session).clone(),
    })?;
    let entry = AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: (*user_id).into(),
        user_id,
        session_id: Some(session_id),
        reason: None,
        ty: AuditLogEntryType::SessionLogin {
            user_id,
            session_id,
        },
        status: AuditLogEntryStatus::Success,
        started_at: session.authorized_at.unwrap_or_else(Time::now_utc),
        ended_at: Time::now_utc(),
        ip_addr: session.imprint.ip_addr.clone(),
        user_agent: session.imprint.user_agent.clone(),
        application_id: session.app_id,
    };
    data.audit_logs_room_append(entry.clone()).await?;
    s.broadcast_room(
        entry.room_id,
        entry.user_id,
        MessageSync::AuditLogEntryCreate { entry },
    )
    .await?;
    Ok(Html(include_str!("../html/oauth.html")))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        // .routes(routes2!(auth_oauth_init))
        .routes(routes2!(auth_oauth_redirect))
}
