//! permission calculations for cached rooms

use common::v1::types::util::Time;
use common::v1::types::{Channel, Permission, PermissionOverwriteType, RoomMember, UserId};
use tracing::warn;

use crate::{services::cache::CachedRoom, types::Permissions};

impl CachedRoom {
    /// query permissions for a room member, optionally in a specific channel
    pub fn query_permissions(&self, user_id: UserId, channel: Option<&Channel>) -> Permissions {
        let member_guard = self.members.get(&user_id);
        let member = member_guard.as_deref();

        // calculate base perms
        let mut perms = self.calculate_room_permissions(user_id, member);

        // admins have full permissions
        if !perms.has(Permission::Admin) {
            if let Some(channel) = channel {
                self.calculate_channel_permissions(&mut perms, channel, member);
            }
        }

        perms
    }

    /// calculate base permissions for a member in a room
    fn calculate_room_permissions(
        &self,
        user_id: UserId,
        member: Option<&RoomMember>,
    ) -> Permissions {
        // owners have full permissions
        if self.room.owner_id == Some(user_id) {
            let mut p = Permissions::empty();
            p.add(Permission::ViewChannel);
            p.add(Permission::Admin);
            return p;
        }

        let Some(member) = member else {
            if self.room.public {
                // use public/default perms
                let everyone_role_id = self.room.id.into_inner().into();
                let mut perms = Permissions::empty();

                if let Some(role) = self.roles.iter().find(|r| r.id == everyone_role_id) {
                    for p in &role.allow {
                        perms.add(*p);
                    }
                    for p in &role.deny {
                        perms.remove(*p);
                    }
                }

                perms.mask(&[Permission::ViewChannel, Permission::ViewAuditLog]);
                return perms;
            } else {
                // the member doesnt exist here; no perms
                return Permissions::empty();
            }
        };

        // calculate role permissions
        let mut allowed = Vec::new();
        let mut denied = Vec::new();

        let everyone_role_id = self.room.id.into_inner().into();

        for role in &self.roles {
            if role.id == everyone_role_id || member.roles.contains(&role.id) {
                allowed.extend_from_slice(&role.allow);
                denied.extend_from_slice(&role.deny);
            }
        }

        let mut perms = Permissions::empty();
        for p in allowed {
            perms.add(p);
        }

        if perms.has(Permission::Admin) {
            return perms;
        }

        for p in denied {
            perms.remove(p);
        }

        // handle timeout
        if let Some(timeout_until) = member.timeout_until {
            if timeout_until > Time::now_utc() {
                perms.set_timed_out(true);
            }
        }

        perms
    }

    /// recursively calculate channel permissions
    fn calculate_channel_permissions(
        &self,
        perms: &mut Permissions,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        if let Some(parent_id) = channel.parent_id {
            if let Some(parent) = self.channels.get(&parent_id) {
                self.calculate_channel_permissions(perms, &parent, member);
            } else {
                warn!(
                    channel_id = ?channel.id,
                    parent_id = ?parent_id,
                    "channel has a parent_id that doesn't exist"
                );
            }
        }

        self.apply_channel_overwrites(perms, &channel, member);
    }

    /// apply the permission overwrites for a channel to a permissions set
    fn apply_channel_overwrites(
        &self,
        perms: &mut Permissions,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        // handle locked channels/threads
        if let Some(locked) = &channel.locked {
            let is_expired = locked.until.is_some_and(|until| until <= Time::now_utc());
            if !is_expired {
                perms.set_channel_locked(true);
                if let Some(member) = member {
                    for role_id in &locked.allow_roles {
                        if member.roles.contains(&(*role_id).into()) {
                            perms.set_locked_bypass(true);
                            break;
                        }
                    }
                }
            }
        }

        if channel.permission_overwrites.is_empty() {
            return;
        }

        // 1. apply everyone allows
        for ow in &channel.permission_overwrites {
            if ow.id != *self.room.id {
                continue;
            }
            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 2. apply everyone denies
        for ow in &channel.permission_overwrites {
            if ow.id != *self.room.id {
                continue;
            }
            for p in &ow.deny {
                perms.remove(*p);
            }
        }

        let Some(member) = member else { return };

        // 3. apply role allows
        for ow in &channel.permission_overwrites {
            if ow.ty != PermissionOverwriteType::Role {
                continue;
            }
            if !member.roles.contains(&ow.id.into()) {
                continue;
            }
            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 4. apply role denies
        for ow in &channel.permission_overwrites {
            if ow.ty != PermissionOverwriteType::Role {
                continue;
            }
            if !member.roles.contains(&ow.id.into()) {
                continue;
            }
            for p in &ow.deny {
                perms.remove(*p);
            }
        }

        // 4. apply user allows
        for ow in &channel.permission_overwrites {
            if ow.ty != PermissionOverwriteType::User {
                continue;
            }
            if ow.id != *member.user_id {
                continue;
            }
            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 4. apply user denies
        for ow in &channel.permission_overwrites {
            if ow.ty != PermissionOverwriteType::User {
                continue;
            }
            if ow.id != *member.user_id {
                continue;
            }
            for p in &ow.deny {
                perms.remove(*p);
            }
        }
    }
}
