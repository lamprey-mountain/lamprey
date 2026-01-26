//! permission calculator

use std::sync::Arc;

use common::v1::types::util::Time;
use common::v1::types::{
    Channel, Permission, PermissionOverwriteType, RoleId, RoomId, RoomMember, UserId,
};
use tracing::warn;

use crate::{services::cache::CachedRoom, types::Permissions};

/// a permission calculator for a room
// NOTE: the only reason why this exists is because accessing a RwLock requires async or blocking_read, which i don't want
// i'd rather be able to implement this directly on CachedRoom, but this works well enough i guess
pub struct PermissionsCalculator {
    pub room_id: RoomId,
    pub owner_id: Option<UserId>,
    pub public: bool,
    pub room: Arc<CachedRoom>,
}

impl PermissionsCalculator {
    /// query permissions for a room member, optionally in a specific channel
    pub fn query(&self, user_id: UserId, channel: Option<&Channel>) -> Permissions {
        let member_guard = self.room.members.get(&user_id);
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
        if self.owner_id == Some(user_id) {
            let mut p = Permissions::empty();
            p.add(Permission::ViewChannel);
            p.add(Permission::Admin);
            return p;
        }

        let Some(member) = member else {
            if self.public {
                // use public/default perms
                let everyone_role_id: RoleId = self.room_id.into_inner().into();
                let mut perms = Permissions::empty();

                if let Some(role) = self.room.roles.iter().find(|r| r.id == everyone_role_id) {
                    for p in &role.allow {
                        perms.add(*p);
                    }
                    for p in &role.deny {
                        perms.remove(*p);
                    }
                }

                perms.set_lurker(true);
                return perms;
            } else {
                // the member doesnt exist here; no perms
                return Permissions::empty();
            }
        };

        // calculate role permissions
        let mut allowed = Vec::new();
        let mut denied = Vec::new();

        let everyone_role_id = self.room_id.into_inner().into();

        for role in &self.room.roles {
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

        // quarantined by automod
        if member.quarantined {
            perms.set_quarantined(true);
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
            if let Some(parent) = self.room.channels.get(&parent_id) {
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
            if ow.id != *self.room_id {
                continue;
            }
            for p in &ow.allow {
                perms.add(*p);
            }
        }

        // 2. apply everyone denies
        for ow in &channel.permission_overwrites {
            if ow.id != *self.room_id {
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

    /// get the rank of this user, the position of the highest role this user has
    pub fn rank(&self, user_id: UserId) -> u64 {
        if self.owner_id == Some(user_id) {
            return u64::MAX;
        }

        let member_guard = self.room.members.get(&user_id);
        let Some(member) = member_guard.as_deref() else {
            // user is not a member, return 0
            return 0;
        };

        let mut rank = 0u64;
        for role_id in &member.roles {
            if let Some(role) = self.room.roles.get(role_id) {
                rank = rank.max(role.position as u64);
            } else {
                warn!(user_id = ?user_id, role_id = ?role_id, "user has role that doesnt exist");
            }
        }

        rank
    }
}
