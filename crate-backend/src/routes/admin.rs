use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use common::v1::types::PaginationQuery;
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageCreate, MessageSync,
    Permission, UserId, SERVER_ROOM_ID, SERVER_USER_ID,
};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};

use crate::error::Result;
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

/// Admin whisper
///
/// send a system dm to one person in particular
#[utoipa::path(
    post,
    path = "/admin/whisper",
    tags = ["admin"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn admin_whisper(
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AdminWhisper>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_room(auth_user_id, SERVER_ROOM_ID).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::Admin)?;

    let changes = Changes::new()
        .add("content", &json.message.content)
        .add("attachments", &json.message.attachments)
        .add("embeds", &json.message.embeds)
        .build();

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user_id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::AdminWhisper {
            user_id: json.user_id,
            changes,
        },
    })
    .await?;

    let (thread, _) = srv.users.init_dm(auth_user_id, json.user_id).await?;
    if !thread.locked {
        d.thread_lock(thread.id).await?;
        srv.threads.invalidate(thread.id).await;
    }

    s.broadcast(MessageSync::ThreadCreate {
        thread: thread.clone(),
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
    tags = ["admin"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn admin_broadcast(
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<AdminBroadcast>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_room(auth_user_id, SERVER_ROOM_ID).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::Admin)?;

    let changes = Changes::new()
        .add("content", &json.message.content)
        .add("attachments", &json.message.attachments)
        .add("embeds", &json.message.embeds)
        .build();

    d.audit_logs_room_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user_id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::AdminBroadcast { changes },
    })
    .await?;

    let mut from = None;
    loop {
        let users = d
            .user_list(PaginationQuery {
                from,
                to: None,
                dir: None,
                limit: Some(1024),
            })
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
                    ss.data().thread_lock(thread.id).await?;
                    srv.threads.invalidate(thread.id).await;
                }

                ss.broadcast(MessageSync::ThreadCreate {
                    thread: thread.clone(),
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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(admin_whisper))
        .routes(routes!(admin_broadcast))
}
