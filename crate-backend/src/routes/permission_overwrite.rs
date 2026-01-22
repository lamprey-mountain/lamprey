use std::{collections::HashSet, sync::Arc};

use crate::routes::util::HeaderReason;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelId, MessageSync,
    Permission, PermissionOverwriteSet, PermissionOverwriteType,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// Permission overwrite
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/permission/{overwrite_id}",
    params(
        ("channel_id", description = "channel id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["channel", "badge.perm.RoleManage"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_overwrite(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path((channel_id, overwrite_id)): Path<(ChannelId, Uuid)>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PermissionOverwriteSet>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let allow_set: HashSet<_> = json.allow.iter().collect();
    let deny_set: HashSet<_> = json.deny.iter().collect();

    if !allow_set.is_disjoint(&deny_set) {
        return Err(Error::BadRequest(
            "a permission cannot be both allowed and denied".to_string(),
        ));
    }

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::RoleManage)?;
    let channel = srv.channels.get(channel_id, None).await?;
    if channel.ty.is_thread() {
        return Err(Error::BadStatic(
            "cant set permission overwrites on threads",
        ));
    }
    if channel.archived_at.is_some() {
        return Err(Error::BadStatic("channel is archived"));
    }
    if channel.deleted_at.is_some() {
        return Err(Error::BadStatic("channel is removed"));
    }
    perms.ensure_unlocked()?;

    if let Some(room_id) = channel.room_id {
        let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
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
        if rank <= other_rank && room.owner_id != Some(auth.user.id) {
            return Err(Error::BadStatic("your rank is too low"));
        }
    } else {
        return Err(Error::BadStatic(
            "cannot set overwrites for channels outside of rooms (eg. direct messages)",
        ));
    }

    // you can't grant/unset/deny permissions you do not have, and if someone else already set them you can't edit them
    let existing = channel
        .permission_overwrites
        .iter()
        .find(|o| o.ty == json.ty && o.id == overwrite_id);

    if existing.is_none()
        && channel.permission_overwrites.len() >= crate::consts::MAX_PERMISSION_OVERWRITES as usize
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
            channel_id,
            overwrite_id,
            json.ty.clone(),
            json.allow.clone(),
            json.deny.clone(),
        )
        .await?;
    srv.channels.invalidate(channel_id).await;
    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason: reason.clone(),
            ty: AuditLogEntryType::PermissionOverwriteSet {
                channel_id,
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

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(channel),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Permission delete
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/permission/{overwrite_id}",
    params(
        ("channel_id", description = "channel id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["channel", "badge.perm.RoleManage"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path((channel_id, overwrite_id)): Path<(ChannelId, Uuid)>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::RoleManage)?;

    let channel = srv.channels.get(channel_id, None).await?;
    if channel.archived_at.is_some() {
        return Err(Error::BadStatic("channel is archived"));
    }
    if channel.deleted_at.is_some() {
        return Err(Error::BadStatic("channel is removed"));
    }
    perms.ensure_unlocked()?;

    if let Some(existing) = channel
        .permission_overwrites
        .iter()
        .find(|o| o.id == overwrite_id)
    {
        if let Some(room_id) = channel.room_id {
            let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
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
            if rank <= other_rank && room.owner_id != Some(auth.user.id) {
                return Err(Error::BadStatic("your rank is too low"));
            }
        } else {
            return Err(Error::BadStatic(
                "cannot set overwrites for channels outside of rooms (eg. direct messages)",
            ));
        }

        for p in &existing.allow {
            perms.ensure(*p)?;
        }
        for p in &existing.deny {
            perms.ensure(*p)?;
        }
    } else {
        return Ok(StatusCode::NO_CONTENT);
    }

    srv.perms
        .permission_overwrite_delete(channel_id, overwrite_id)
        .await?;
    srv.channels.invalidate(channel_id).await;
    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason: reason.clone(),
            ty: AuditLogEntryType::PermissionOverwriteDelete {
                channel_id,
                overwrite_id,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(channel),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(permission_overwrite))
        .routes(routes!(permission_delete))
}
