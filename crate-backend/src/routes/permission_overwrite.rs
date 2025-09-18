use std::{collections::HashSet, sync::Arc};

use crate::routes::util::HeaderReason;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, Permission,
    PermissionOverwrite, PermissionOverwriteSet, PermissionOverwriteType, ThreadId,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path((thread_id, overwrite_id)): Path<(ThreadId, Uuid)>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PermissionOverwriteSet>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let thread = srv.threads.get(thread_id, None).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    if let Some(room_id) = thread.room_id {
        let rank = srv.perms.get_user_rank(room_id, auth_user.id).await?;
        let other_rank = match json.ty {
            PermissionOverwriteType::Role => {
                let role = s.data().role_select(room_id, overwrite_id.into()).await?;
                role.position
            }
            PermissionOverwriteType::User => {
                srv.perms
                    .get_user_rank(room_id, overwrite_id.into())
                    .await?
            }
        };
        let room = srv.rooms.get(room_id, None).await?;
        if rank <= other_rank && room.owner_id != Some(auth_user.id) {
            return Err(Error::BadStatic("your rank is too low"));
        }
    } else {
        return Err(Error::BadStatic(
            "cannot set overwrites for threads outside of rooms (eg. direct messages)",
        ));
    }

    // you can't grant/unset/deny permissions you do not have, and if someone else already set them you can't edit them
    let existing = thread
        .permission_overwrites
        .iter()
        .find(|o| o.ty == json.ty && o.id == overwrite_id);

    if existing.is_none()
        && thread.permission_overwrites.len() >= crate::consts::MAX_PERMISSION_OVERWRITES as usize
    {
        return Err(Error::BadRequest(format!(
            "too many permission overwrites (max {})",
            crate::consts::MAX_PERMISSION_OVERWRITES
        )));
    }

    if let Some(existing) = &existing {
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
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;

    if let Some(room_id) = thread.room_id {
        s.data()
            .audit_logs_room_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::ThreadOverwriteSet {
                    thread_id,
                    overwrite_id,
                    ty: json.ty,
                    changes: if let Some(existing) = &existing {
                        Changes::new()
                            .change("allow", &existing.allow, &json.allow)
                            .change("deny", &existing.deny, &json.deny)
                            .build()
                    } else {
                        Changes::new()
                            .add("allow", &json.allow)
                            .add("deny", &json.deny)
                            .build()
                    },
                },
            })
            .await?;
    }

    s.broadcast_thread(
        thread_id,
        auth_user.id,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path((thread_id, overwrite_id)): Path<(ThreadId, Uuid)>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;

    let thread = srv.threads.get(thread_id, None).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    if let Some(existing) = thread
        .permission_overwrites
        .iter()
        .find(|o| o.id == overwrite_id)
    {
        if let Some(room_id) = thread.room_id {
            let rank = srv.perms.get_user_rank(room_id, auth_user.id).await?;
            let other_rank = match existing.ty {
                PermissionOverwriteType::Role => {
                    let role = s.data().role_select(room_id, overwrite_id.into()).await?;
                    role.position
                }
                PermissionOverwriteType::User => {
                    srv.perms
                        .get_user_rank(room_id, overwrite_id.into())
                        .await?
                }
            };
            let room = srv.rooms.get(room_id, None).await?;
            if rank <= other_rank && room.owner_id != Some(auth_user.id) {
                return Err(Error::BadStatic("your rank is too low"));
            }
        } else {
            return Err(Error::BadStatic(
                "cannot set overwrites for threads outside of rooms (eg. direct messages)",
            ));
        }

        for p in &existing.allow {
            perms.ensure(*p)?;
        }
        for p in &existing.deny {
            perms.ensure(*p)?;
        }
    } else {
        return Ok(());
    }

    srv.perms
        .permission_overwrite_delete(thread_id, overwrite_id)
        .await?;
    srv.threads.invalidate(thread_id).await;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;

    if let Some(room_id) = thread.room_id {
        s.data()
            .audit_logs_room_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
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
        auth_user.id,
        MessageSync::ThreadUpdate { thread },
    )
    .await?;
    Ok(())
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
    Auth(_auth_user): Auth,
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
    Auth(_auth_user): Auth,
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
