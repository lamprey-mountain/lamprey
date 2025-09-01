use std::{collections::HashSet, sync::Arc};

use crate::routes::util::HeaderReason;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, Permission,
    PermissionOverwrite, PermissionOverwriteSet, ThreadId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// Thread permission overwrite
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/permission/{overwrite_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_thread_overwrite(
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Path((thread_id, overwrite_id)): Path<(ThreadId, Uuid)>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PermissionOverwriteSet>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let thread = srv.threads.get(thread_id, None).await?;

    // you can't grant/unset/deny permissions you do not have, and if someone else already set them you can't edit them
    let existing = thread
        .permission_overwrites
        .iter()
        .find(|o| o.ty == json.ty && o.id == overwrite_id);
    if let Some(existing) = existing {
        let ea: HashSet<Permission> = existing.allow.iter().cloned().collect();
        let ed: HashSet<Permission> = existing.deny.iter().cloned().collect();
        let ja: HashSet<Permission> = json.allow.iter().cloned().collect();
        let jd: HashSet<Permission> = json.deny.iter().cloned().collect();

        // must have permission to add/remove allows
        for p in ea.symmetric_difference(&ja) {
            perms.ensure(*p)?;
        }

        // must have permission to add/remove denies
        for p in ed.symmetric_difference(&jd) {
            perms.ensure(*p)?;
        }
    } else {
        for p in &json.allow {
            perms.ensure(*p)?;
        }
        for p in &json.deny {
            perms.ensure(*p)?;
        }
    }

    let thread = srv.threads.get(thread_id, Some(auth_user_id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }

    srv.perms
        .permission_overwrite_upsert(
            thread_id,
            overwrite_id,
            json.ty.clone(),
            json.allow.clone(),
            json.deny.clone(),
        )
        .await?;
    srv.threads.invalidate(thread_id).await;
    let thread = srv.threads.get(thread_id, Some(auth_user_id)).await?;

    if let Some(room_id) = thread.room_id {
        s.data()
            .audit_logs_room_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user_id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::ThreadOverwriteSet {
                    thread_id,
                    overwrite_id,
                    ty: json.ty,
                    allow: json.allow,
                    deny: json.deny,
                },
            })
            .await?;
    }

    s.broadcast_thread(
        thread_id,
        auth_user_id,
        MessageSync::ThreadUpdate { thread },
    )
    .await?;
    Ok(())
}

/// Thread permission delete
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/permission/{overwrite_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_thread_delete(
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Path((thread_id, overwrite_id)): Path<(ThreadId, Uuid)>,
    HeaderReason(reason): HeaderReason,
) -> Result<Json<()>> {
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;

    let thread = srv.threads.get(thread_id, None).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }

    if let Some(existing) = thread
        .permission_overwrites
        .iter()
        .find(|o| o.id == overwrite_id)
    {
        for p in &existing.allow {
            perms.ensure(*p)?;
        }
        for p in &existing.deny {
            perms.ensure(*p)?;
        }
    }

    srv.perms
        .permission_overwrite_delete(thread_id, overwrite_id)
        .await?;
    srv.threads.invalidate(thread_id).await;
    let thread = srv.threads.get(thread_id, Some(auth_user_id)).await?;

    if let Some(room_id) = thread.room_id {
        s.data()
            .audit_logs_room_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user_id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::ThreadOverwriteDelete {
                    thread_id,
                    overwrite_id,
                },
            })
            .await?;
    }

    s.broadcast_thread(
        thread_id,
        auth_user_id,
        MessageSync::ThreadUpdate { thread },
    )
    .await?;
    Ok(Json(()))
}

/// Tag permission override upsert (TODO)
///
/// Upsert a tag permission override
#[utoipa::path(
    put,
    path = "/room/{room_id}/tag/{tag_id}/permission/{overwrite_id}",
    params(
        ("room_id", description = "Room id"),
        ("tag_id", description = "Tag id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["tag"],
    responses((status = OK, body = PermissionOverwrite, description = "success"))
)]
async fn permission_tag_overwrite(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<PermissionOverwrite>,
) -> Result<Json<PermissionOverwrite>> {
    Err(Error::Unimplemented)
}

/// Tag permission override delete (TODO)
///
/// Delete a tag permission override
#[utoipa::path(
    delete,
    path = "/room/{room_id}/tag/{tag_id}/permission/{overwrite_id}",
    params(
        ("room_id", description = "Room id"),
        ("tag_id", description = "Tag id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["tag"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_tag_delete(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(permission_thread_overwrite))
        .routes(routes!(permission_thread_delete))
        .routes(routes!(permission_tag_overwrite))
        .routes(routes!(permission_tag_delete))
}
