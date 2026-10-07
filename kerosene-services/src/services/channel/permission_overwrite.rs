use std::collections::HashSet;

use crate::prelude::*;
use crate::services::channel::ServiceChannels;
use common::v1::types::{AuditLogEntryType, MessageSync};
use common::v1::types::{
    ChannelId, Permission, PermissionOverwriteSet, PermissionOverwriteType, util::Changes,
};
use common::v2::types::{PermissionOverwriteId, UserId};
use kerosene_core::error::{ApiError, ErrorCode, LegacyErrorExt, ServerResult};
use kerosene_core::types::auth::{Auth5, Auth5Ext};

impl ServiceChannels {
    /// create or update a permission overwrite for a channel
    pub async fn permission_set<A: Auth5>(
        &self,
        auth: &mut A,
        channel_id: ChannelId,
        overwrite_id: PermissionOverwriteId,
        payload: &PermissionOverwriteSet,
    ) -> ServerResult<()> {
        let user = auth.ensure_user()?;
        let user_id = user.id;

        // permissions can't be allowed and denied at the same time
        let allow_set: HashSet<_> = payload.allow.iter().collect();
        let deny_set: HashSet<_> = payload.deny.iter().collect();
        if !allow_set.is_disjoint(&deny_set) {
            return Err(ApiError::from_code(ErrorCode::PermissionConflict).into());
        }

        let srv = self.globals.services();
        let mut perms = srv
            .perms
            .for_channel3(Some(user_id), channel_id)
            .await
            .cast_internal()?
            .ensure_view()?;
        perms.needs(Permission::RoleManage);

        // load channel and room
        let channel = srv.channels.get(channel_id, None).await.cast_internal()?;
        let room_id = if let Some(room_id) = channel.room_id {
            room_id
        } else {
            return Err(
                ApiError::from_code(ErrorCode::CannotSetPermissionsOnThisChannelType).into(),
            );
        };
        let room_handle = srv.rooms.load(room_id);
        let room = room_handle.ready(false).await.cast_internal()?;

        // only top level channels can have overwrites
        if room.channels.contains_key(&channel.id) {
            return Err(
                ApiError::from_code(ErrorCode::CannotSetPermissionsOnThisChannelType).into(),
            );
        }

        channel.ensure_unarchived()?;
        channel.ensure_unremoved()?;
        perms.needs_unlocked();

        // check rank
        let rank = perms.rank();
        let other_rank = match payload.ty {
            PermissionOverwriteType::Role => {
                let role = room
                    .roles
                    .get(&(*overwrite_id).into())
                    .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownRole))?;
                role.position
            }
            PermissionOverwriteType::User => srv
                .perms
                .get_user_rank(room_id, (*overwrite_id).into())
                .await
                .cast_internal()?,
        };
        if rank <= other_rank && room.room.owner_id != Some(user_id) {
            return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
        }

        let existing = channel
            .permission_overwrites
            .iter()
            .find(|o| o.ty == payload.ty && *o.id == *overwrite_id);

        let max_overwrites = crate::consts::MAX_PERMISSION_OVERWRITES as usize;
        if existing.is_none() && channel.permission_overwrites.len() >= max_overwrites {
            // TODO: specific error code
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                format!("maximum number of permission overwrites reached for this channel ({max_overwrites})"),
            )
            .into());
        }

        // you can only allow or deny permissions you have
        if let Some(existing) = &existing {
            let ea: HashSet<Permission> = existing.allow.iter().cloned().collect();
            let ed: HashSet<Permission> = existing.deny.iter().cloned().collect();
            let ja: HashSet<Permission> = payload.allow.iter().cloned().collect();
            let jd: HashSet<Permission> = payload.deny.iter().cloned().collect();

            for p in ea.symmetric_difference(&ja) {
                perms.needs(*p);
            }

            for p in ed.symmetric_difference(&jd) {
                perms.needs(*p);
            }
        } else {
            for p in &payload.allow {
                perms.needs(*p);
            }
            for p in &payload.deny {
                perms.needs(*p);
            }
        }
        perms.check()?;

        srv.perms
            .permission_overwrite_upsert(
                channel_id,
                *overwrite_id,
                payload.ty.clone(),
                payload.allow.clone(),
                payload.deny.clone(),
            )
            .await
            .cast_internal()?;
        srv.channels.invalidate(channel_id).await;

        // reload channel
        let channel = srv
            .channels
            .get(channel_id, Some(user_id))
            .await
            .cast_internal()?;

        if let Some(room_id) = channel.room_id {
            auth.set_room_id(room_id);
            let audit_log_entry = if let Some(existing) = existing {
                AuditLogEntryType::PermissionOverwriteUpdate {
                    channel_id,
                    overwrite_id: *overwrite_id,
                    ty: payload.ty.clone(),
                    changes: Changes::new()
                        .change("allow", &existing.allow, &payload.allow)
                        .change("deny", &existing.deny, &payload.deny)
                        .build(),
                }
            } else {
                AuditLogEntryType::PermissionOverwriteCreate {
                    channel_id,
                    overwrite_id: *overwrite_id,
                    ty: payload.ty.clone(),
                    changes: Changes::new()
                        .add("allow", &payload.allow)
                        .add("deny", &payload.deny)
                        .build(),
                }
            };
            auth.al_push(audit_log_entry);
        }

        self.globals
            .messaging()
            .broadcast_channel(
                channel_id,
                MessageSync::ChannelUpdate {
                    channel: Box::new(channel),
                },
            )
            .await
            .cast_internal()?;

        Ok(())
    }

    /// delete a permission overwrite for a channel
    pub async fn permission_delete<A: Auth5>(
        &self,
        auth: &mut A,
        channel_id: ChannelId,
        overwrite_id: PermissionOverwriteId,
    ) -> ServerResult<()> {
        let user = auth.ensure_user()?;
        let user_id = user.id;

        let srv = self.globals.services();
        let mut perms = srv
            .perms
            .for_channel3(Some(user_id), channel_id)
            .await
            .cast_internal()?
            .ensure_view()?;
        perms.needs(Permission::RoleManage);

        let channel = srv.channels.get(channel_id, None).await.cast_internal()?;
        channel.ensure_unarchived()?;
        channel.ensure_unremoved()?;
        perms.needs_unlocked();

        let existing = if let Some(existing) = channel
            .permission_overwrites
            .iter()
            .find(|o| *o.id == *overwrite_id)
        {
            if let Some(room_id) = channel.room_id {
                let room_handle = srv.rooms.load(room_id);
                let room = room_handle.ready(false).await.cast_internal()?;

                // check rank
                let rank = perms.rank();
                let other_rank = match existing.ty {
                    PermissionOverwriteType::Role => {
                        let role = room
                            .roles
                            .get(&(*overwrite_id).into())
                            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownRole))?;
                        role.position
                    }
                    PermissionOverwriteType::User => srv
                        .perms
                        .get_user_rank(room_id, (*overwrite_id).into())
                        .await
                        .cast_internal()?,
                };
                if rank <= other_rank && room.room.owner_id != Some(user_id) {
                    return Err(ApiError::from_code(ErrorCode::InsufficientRank).into());
                }
            } else {
                return Err(
                    ApiError::from_code(ErrorCode::CannotSetPermissionsOnThisChannelType).into(),
                );
            }

            for p in &existing.allow {
                perms.needs(*p);
            }
            for p in &existing.deny {
                perms.needs(*p);
            }
            perms.check()?;
            existing
        } else {
            return Ok(());
        };

        srv.perms
            .permission_overwrite_delete(channel_id, *overwrite_id)
            .await
            .cast_internal()?;
        srv.channels.invalidate(channel_id).await;

        // reload channel
        let channel = srv
            .channels
            .get(channel_id, Some(user_id))
            .await
            .cast_internal()?;

        if let Some(room_id) = channel.room_id {
            auth.set_room_id(room_id);
            auth.al_push(AuditLogEntryType::PermissionOverwriteDelete {
                channel_id,
                overwrite_id: *overwrite_id,
                ty: existing.ty,
            });
        }

        self.globals
            .messaging()
            .broadcast_channel(
                channel_id,
                MessageSync::ChannelUpdate {
                    channel: Box::new(channel),
                },
            )
            .await
            .cast_internal()?;

        Ok(())
    }
}
