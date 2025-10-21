use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use common::v1::types::{
    util::{Changes, Time},
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageCreate, MessageSync, Permission,
    UserId, SERVER_ROOM_ID, SERVER_USER_ID,
};
use common::v1::types::{
    ChannelPatch, PaginationQuery, SessionStatus, SessionType, SessionWithToken,
};
use http::StatusCode;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::util::{Auth, HeaderReason};

use crate::error::Result;
use crate::types::{DbSessionCreate, DbUserCreate, SessionToken};
use crate::ServerState;

#[derive(Deserialize, ToSchema)]
struct AdminWhisper {
    user_id: UserId,
    message: MessageCreate,
}

#[derive(Deserialize, ToSchema)]
struct AdminBroadcast {
    message: MessageCreate,
}

#[derive(Deserialize, ToSchema)]
struct AdminRegisterUser {
    user_id: UserId,
}

/// Admin whisper
///
/// send a system dm to one person in particular
#[utoipa::path(
    post,
    path = "/admin/whisper",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn admin_whisper(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AdminWhisper>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;

    perms.ensure(Permission::Admin)?;

    let changes = Changes::new()
        .add("content", &json.message.content)
        .add("attachments", &json.message.attachments)
        .add("embeds", &json.message.embeds)
        .build();

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::AdminWhisper {
            user_id: json.user_id,
            changes,
        },
    })
    .await?;

    let (thread, _) = srv.users.init_dm(auth_user.id, json.user_id).await?;
    if !thread.locked {
        d.channel_update(
            thread.id,
            ChannelPatch {
                locked: Some(true),
                ..Default::default()
            },
        )
        .await?;
        srv.channels.invalidate(thread.id).await;
    }

    s.broadcast(MessageSync::ChannelCreate {
        channel: Box::new(thread.clone()),
    })?;

    srv.messages
        .create(thread.id, SERVER_USER_ID, None, None, json.message)
        .await?;

    Ok(())
}

/// Admin broadcast
///
/// send a system dm to everyone on the server
#[utoipa::path(
    post,
    path = "/admin/broadcast",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn admin_broadcast(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AdminBroadcast>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;

    perms.ensure(Permission::Admin)?;

    let changes = Changes::new()
        .add("content", &json.message.content)
        .add("attachments", &json.message.attachments)
        .add("embeds", &json.message.embeds)
        .build();

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::AdminBroadcast { changes },
    })
    .await?;

    let mut from = None;
    loop {
        let users = d
            .user_list(
                PaginationQuery {
                    from,
                    to: None,
                    dir: None,
                    limit: Some(1024),
                },
                None,
            )
            .await?;
        let Some(last) = users.items.last() else {
            break;
        };
        from = Some(last.id);
        for user in users.items {
            // NOTE: do i really want to be cloning this potentially hundreds to thousands of times?
            let msg = json.message.clone();
            let ss = s.clone();
            tokio::spawn(async move {
                let srv = ss.services();
                let (thread, _) = srv.users.init_dm(SERVER_USER_ID, user.id).await?;
                if !thread.locked {
                    ss.data()
                        .channel_update(
                            thread.id,
                            ChannelPatch {
                                locked: Some(true),
                                ..Default::default()
                            },
                        )
                        .await?;
                    srv.channels.invalidate(thread.id).await;
                }

                ss.broadcast(MessageSync::ChannelCreate {
                    channel: Box::new(thread.clone()),
                })?;

                srv.messages
                    .create(thread.id, SERVER_USER_ID, None, None, msg)
                    .await?;

                Result::Ok(())
            });
        }
        if !users.has_more {
            break;
        }
    }

    Ok(())
}

/// Admin register user
///
/// Registers an existing guest user, promoting them to a regular user.
/// Bypasses the normal invite/auth method flow.
#[utoipa::path(
    post,
    path = "/admin/register-user",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin"],
    request_body = AdminRegisterUser,
    responses((status = OK, description = "User registered", body = SessionWithToken))
)]
async fn admin_register_user(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AdminRegisterUser>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
    perms.ensure(Permission::Admin)?;

    let target_user_id = json.user_id;

    d.user_set_registered(target_user_id, Some(Time::now_utc()), None)
        .await?;

    srv.users.invalidate(target_user_id).await;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason: reason,
        ty: AuditLogEntryType::UserRegistered {
            user_id: target_user_id,
        },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(admin_whisper))
        .routes(routes!(admin_broadcast))
        .routes(routes!(admin_register_user))
}
