use std::sync::Arc;

use axum::extract::Path;
use axum::{extract::State, response::IntoResponse, Json};
use common::v1::types::PaginationQuery;
use common::v1::types::{
    util::{Changes, Time},
    AuditLogEntryType, MessageSync, Permission, SERVER_ROOM_ID, SERVER_USER_ID,
};
use http::StatusCode;
use lamprey_backend_core::types::admin::{
    AdminBroadcast, AdminCollectGarbage, AdminCollectGarbageResponse, AdminPurgeCache,
    AdminPurgeCacheResponse, AdminRegisterUser, AdminWhisper,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;

use crate::{error::Result, ServerState};
use common::v1::types::ChannelId;

/// Admin whisper
///
/// send a system dm to one person in particular
#[utoipa::path(
    post,
    path = "/admin/whisper",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin", "badge.audit-log.AdminWhisper"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn admin_whisper(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AdminWhisper>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();

    let perms = srv.perms.for_server(auth.user.id).await?;

    perms.ensure(Permission::Admin)?;

    let changes = Changes::new()
        .add("content", &json.message.content)
        .add("attachments", &json.message.attachments)
        .add("embeds", &json.message.embeds)
        .build();

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::AdminWhisper {
        user_id: json.user_id,
        changes,
    })
    .await?;

    let (thread, _) = srv.users.init_dm(auth.user.id, json.user_id, true).await?;

    s.broadcast(MessageSync::ChannelCreate {
        channel: Box::new(thread.clone()),
    })?;

    srv.messages
        .create_system(thread.id, SERVER_USER_ID, None, json.message)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Admin broadcast
///
/// send a system dm to everyone on the server
#[utoipa::path(
    post,
    path = "/admin/broadcast",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin", "badge.audit-log.AdminBroadcast"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn admin_broadcast(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AdminBroadcast>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_server(auth.user.id).await?;

    perms.ensure(Permission::Admin)?;

    let changes = Changes::new()
        .add("content", &json.message.content)
        .add("attachments", &json.message.attachments)
        .add("embeds", &json.message.embeds)
        .build();

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::AdminBroadcast { changes })
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
                let (thread, _) = srv.users.init_dm(SERVER_USER_ID, user.id, true).await?;

                ss.broadcast(MessageSync::ChannelCreate {
                    channel: Box::new(thread.clone()),
                })?;

                srv.messages
                    .create_system(thread.id, SERVER_USER_ID, None, msg)
                    .await?;

                Result::Ok(())
            });
        }
        if !users.has_more {
            break;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Admin register user
///
/// Registers an existing guest user, promoting them to a regular user.
/// Bypasses the normal invite/auth method flow.
#[utoipa::path(
    post,
    path = "/admin/register-user",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin", "badge.audit-log.UserRegistered"],
    request_body = AdminRegisterUser,
    responses((status = NO_CONTENT, description = "User registered"))
)]
async fn admin_register_user(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AdminRegisterUser>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let d = s.data();

    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    let target_user_id = json.user_id;

    d.user_set_registered(target_user_id, Some(Time::now_utc()), None)
        .await?;

    srv.users.invalidate(target_user_id).await;

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::UserRegistered {
        user_id: target_user_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Admin purge cache
#[utoipa::path(
    post,
    path = "/admin/purge-cache",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin"],
    request_body = AdminPurgeCache,
    responses(
        (status = OK, body = AdminPurgeCacheResponse, description = "cache purging task finished"),
    ),
)]
async fn admin_purge_cache(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AdminPurgeCache>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();

    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    let res = srv.admin.purge_caches(json).await?;
    Ok(Json(res))
}

/// Admin collect garbage
#[utoipa::path(
    post,
    path = "/admin/collect-garbage",
    tags = ["admin", "badge.admin_only", "badge.perm.Admin"],
    request_body = AdminCollectGarbage,
    responses(
        (status = ACCEPTED, description = "garbage collecting task started"),
        (status = OK, body = AdminCollectGarbageResponse, description = "garbage collecting task finished"),
    )
)]
async fn admin_collect_garbage(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AdminCollectGarbage>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();

    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    let res = srv.admin.collect_garbage(json).await?;
    Ok(Json(res))
}

/// Admin reindex channel
///
/// Queue a channel to be reindexed for search
#[utoipa::path(
    post,
    path = "/admin/reindex-channel/{channel_id}",
    tags = ["admin", "badge.admin_only", "badge.server-perm.Admin", "badge.audit-log.ChannelReindex"],
    params(("channel_id" = String, Path, description = "Channel id to reindex")),
    responses(
        (status = ACCEPTED, description = "Channel reindexing queued"),
    )
)]
async fn admin_reindex_channel(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(channel_id): Path<ChannelId>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let al = auth.audit_log(SERVER_ROOM_ID);
    let srv = s.services();

    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    srv.admin.reindex_channel(channel_id).await?;

    al.commit_success(AuditLogEntryType::ChannelReindex { channel_id })
        .await?;

    Ok(StatusCode::ACCEPTED)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(admin_whisper))
        .routes(routes!(admin_broadcast))
        .routes(routes!(admin_register_user))
        .routes(routes!(admin_purge_cache))
        .routes(routes!(admin_collect_garbage))
        .routes(routes!(admin_reindex_channel))
}
