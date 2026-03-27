use std::{collections::HashSet, sync::Arc};

use axum::extract::State;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{
    util::Changes, AuditLogEntryType, MessageSync, Permission, PermissionOverwriteType,
};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Permission overwrite
#[handler(routes::permission_set)]
async fn permission_set(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::permission_set::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let allow_set: HashSet<_> = req.overwrite.allow.iter().collect();
    let deny_set: HashSet<_> = req.overwrite.deny.iter().collect();

    if !allow_set.is_disjoint(&deny_set) {
        return Err(ApiError::from_code(ErrorCode::PermissionConflict).into());
    }

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    perms.ensure(Permission::RoleManage)?;
    let channel = srv.channels.get(req.channel_id, None).await?;
    if channel.is_thread() {
        return Err(ApiError::from_code(ErrorCode::CannotSetPermissionsOnThisChannelType).into());
    }
    channel.ensure_unarchived()?;
    channel.ensure_unremoved()?;
    perms.ensure_unlocked()?;

    if let Some(room_id) = channel.room_id {
        let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
        let other_rank = match req.overwrite.ty {
            PermissionOverwriteType::Role => {
                let role = s
                    .data()
                    .role_select(room_id, (*req.overwrite_id).into())
                    .await?;
                role.position
            }
            PermissionOverwriteType::User => {
                srv.perms
                    .get_user_rank(room_id, (*req.overwrite_id).into())
                    .await?
            }
        };
        let room = srv.rooms.get(room_id, None).await?;
        if rank <= other_rank && room.owner_id != Some(auth.user.id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }
    } else {
        return Err(ApiError::from_code(ErrorCode::CannotSetPermissionsOnThisChannelType).into());
    }

    let existing = channel
        .permission_overwrites
        .iter()
        .find(|o| o.ty == req.overwrite.ty && o.id == *req.overwrite_id);

    if existing.is_none()
        && channel.permission_overwrites.len() >= crate::consts::MAX_PERMISSION_OVERWRITES as usize
    {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }

    if let Some(existing) = &existing {
        let ea: HashSet<Permission> = existing.allow.iter().cloned().collect();
        let ed: HashSet<Permission> = existing.deny.iter().cloned().collect();
        let ja: HashSet<Permission> = req.overwrite.allow.iter().cloned().collect();
        let jd: HashSet<Permission> = req.overwrite.deny.iter().cloned().collect();

        for p in ea.symmetric_difference(&ja) {
            perms.ensure(*p)?;
        }

        for p in ed.symmetric_difference(&jd) {
            perms.ensure(*p)?;
        }
    } else {
        for p in &req.overwrite.allow {
            perms.ensure(*p)?;
        }
        for p in &req.overwrite.deny {
            perms.ensure(*p)?;
        }
    }

    srv.perms
        .permission_overwrite_upsert(
            req.channel_id,
            *req.overwrite_id,
            req.overwrite.ty.clone(),
            req.overwrite.allow.clone(),
            req.overwrite.deny.clone(),
        )
        .await?;
    srv.channels.invalidate(req.channel_id).await;
    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        let audit_log_entry = if existing.is_some() {
            AuditLogEntryType::PermissionOverwriteUpdate {
                channel_id: req.channel_id,
                overwrite_id: *req.overwrite_id,
                ty: req.overwrite.ty,
                changes: Changes::new()
                    .change(
                        "allow",
                        &existing.as_ref().unwrap().allow,
                        &req.overwrite.allow,
                    )
                    .change(
                        "deny",
                        &existing.as_ref().unwrap().deny,
                        &req.overwrite.deny,
                    )
                    .build(),
            }
        } else {
            AuditLogEntryType::PermissionOverwriteCreate {
                channel_id: req.channel_id,
                overwrite_id: *req.overwrite_id,
                ty: req.overwrite.ty,
                changes: Changes::new()
                    .add("allow", &req.overwrite.allow)
                    .add("deny", &req.overwrite.deny)
                    .build(),
            }
        };
        al.commit_success(audit_log_entry).await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(channel),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Permission delete
#[handler(routes::permission_remove)]
async fn permission_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::permission_remove::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;
    perms.ensure(Permission::RoleManage)?;

    let channel = srv.channels.get(req.channel_id, None).await?;
    channel.ensure_unarchived()?;
    channel.ensure_unremoved()?;
    perms.ensure_unlocked()?;

    let existing = if let Some(existing) = channel
        .permission_overwrites
        .iter()
        .find(|o| o.id == *req.overwrite_id)
    {
        if let Some(room_id) = channel.room_id {
            let rank = srv.perms.get_user_rank(room_id, auth.user.id).await?;
            let other_rank = match existing.ty {
                PermissionOverwriteType::Role => {
                    let role = s
                        .data()
                        .role_select(room_id, (*req.overwrite_id).into())
                        .await?;
                    role.position
                }
                PermissionOverwriteType::User => {
                    srv.perms
                        .get_user_rank(room_id, (*req.overwrite_id).into())
                        .await?
                }
            };
            let room = srv.rooms.get(room_id, None).await?;
            if rank <= other_rank && room.owner_id != Some(auth.user.id) {
                return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
            }
        } else {
            return Err(
                ApiError::from_code(ErrorCode::CannotSetPermissionsOnThisChannelType).into(),
            );
        }

        for p in &existing.allow {
            perms.ensure(*p)?;
        }
        for p in &existing.deny {
            perms.ensure(*p)?;
        }
        existing
    } else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };

    srv.perms
        .permission_overwrite_delete(req.channel_id, *req.overwrite_id)
        .await?;
    srv.channels.invalidate(req.channel_id).await;
    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::PermissionOverwriteDelete {
            channel_id: req.channel_id,
            overwrite_id: *req.overwrite_id,
            ty: existing.ty,
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(channel),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(permission_set))
        .routes(routes2!(permission_remove))
}
