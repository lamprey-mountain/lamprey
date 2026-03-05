//! permission calculator

use std::sync::Arc;

use common::v1::types::util::Time;
use common::v1::types::{
    Channel, Permission, PermissionOverwriteType, RoleId, RoomId, RoomMember, UserId,
};
use tracing::{trace, warn};

use crate::{
    services::cache::{CachedChannel, CachedRoom},
    types::{PermissionBits, Permissions},
};

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
        let member = member_guard.as_deref().map(|m| &m.member);

        // calculate base perms
        let mut perms = self.calculate_room_permissions(user_id, member);

        // admins have full permissions
        if !perms.has(Permission::Admin) {
            if let Some(channel) = channel {
                // only calculate channel permissions if the channel exists in cache
                // (channels not in cache have no overwrites)
                if let Some(cached_channel) = self.room.channels.get(&channel.id) {
                    self.calculate_channel_permissions(&mut perms, &cached_channel, member);
                }
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
            p.set_is_room_member(true);
            p.add(Permission::ViewChannel);
            p.add(Permission::Admin);
            return p;
        }

        let Some(member) = member else {
            if self.public {
                // use public/default perms
                let everyone_role_id: RoleId = self.room_id.into_inner().into();
                let mut perms = Permissions::empty();
                perms.set_is_room_member(false);

                if let Some(role) = self
                    .room
                    .roles
                    .iter()
                    .find(|r| r.inner.id == everyone_role_id)
                {
                    perms.add_bits(role.allow);
                    perms.remove_bits(role.deny);
                }

                perms.set_lurker(true);
                return perms;
            } else {
                // the member doesnt exist here and room not public; no perms
                tracing::debug!(?user_id, room_id = ?self.room_id, "user not a member and room not public");
                return Permissions::empty();
            }
        };

        // calculate role permissions using bit operations
        let mut allowed_bits = PermissionBits::default();
        let mut denied_bits = PermissionBits::default();

        let everyone_role_id = self.room_id.into_inner().into();

        for role in &self.room.roles {
            if role.inner.id == everyone_role_id || member.roles.contains(&role.inner.id) {
                allowed_bits.add_all(role.allow);
                denied_bits.add_all(role.deny);
            }
        }

        let mut perms = Permissions::empty();
        perms.set_is_room_member(true);
        perms.add_bits(allowed_bits);

        trace!(?user_id, room_id = ?self.room_id, bits = ?allowed_bits, "calculated base perms");

        if perms.has(Permission::Admin) {
            return perms;
        }

        perms.remove_bits(denied_bits);
        trace!(?user_id, room_id = ?self.room_id, "perms after denied bits: {:?}", perms);

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
        cc: &CachedChannel,
        member: Option<&RoomMember>,
    ) {
        if let Some(parent_id) = cc.inner.parent_id {
            if let Some(parent) = self.room.channels.get(&parent_id) {
                self.calculate_channel_permissions(perms, &parent, member);
            } else {
                warn!(
                    channel_id = ?cc.inner.id,
                    parent_id = ?parent_id,
                    "channel has a parent_id that doesn't exist"
                );
            }
        }

        self.apply_channel_overwrites(perms, cc, member);
    }

    /// apply the permission overwrites for a channel to a permissions set
    fn apply_channel_overwrites(
        &self,
        perms: &mut Permissions,
        cc: &CachedChannel,
        member: Option<&RoomMember>,
    ) {
        // handle locked channels/threads
        if let Some(locked) = &cc.inner.locked {
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

        if cc.overwrites.is_empty() {
            return;
        }

        // 1. apply everyone allows
        for ow in &cc.overwrites {
            if ow.id != *self.room_id {
                continue;
            }
            perms.add_bits(ow.allow);
        }

        // 2. apply everyone denies
        for ow in &cc.overwrites {
            if ow.id != *self.room_id {
                continue;
            }
            perms.remove_bits(ow.deny);
        }

        let Some(member) = member else { return };

        // 3. apply role allows
        for ow in &cc.overwrites {
            if ow.ty != PermissionOverwriteType::Role {
                continue;
            }
            if !member.roles.contains(&ow.id.into()) {
                continue;
            }
            perms.add_bits(ow.allow);
        }

        // 4. apply role denies
        for ow in &cc.overwrites {
            if ow.ty != PermissionOverwriteType::Role {
                continue;
            }
            if !member.roles.contains(&ow.id.into()) {
                continue;
            }
            perms.remove_bits(ow.deny);
        }

        // 5. apply user allows
        for ow in &cc.overwrites {
            if ow.ty != PermissionOverwriteType::User {
                continue;
            }
            if ow.id != *member.user_id {
                continue;
            }
            perms.add_bits(ow.allow);
        }

        // 6. apply user denies
        for ow in &cc.overwrites {
            if ow.ty != PermissionOverwriteType::User {
                continue;
            }
            if ow.id != *member.user_id {
                continue;
            }
            perms.remove_bits(ow.deny);
        }
    }

    /// get the rank of this user, the position of the highest role this user has
    pub fn rank(&self, user_id: UserId) -> u64 {
        if self.owner_id == Some(user_id) {
            return u64::MAX;
        }

        let member_guard = self.room.members.get(&user_id);
        let Some(member) = member_guard.as_deref().map(|m| &m.member) else {
            // user is not a member, return 0
            return 0;
        };

        let mut rank = 0u64;
        for role_id in &member.roles {
            if let Some(role) = self.room.roles.get(role_id) {
                rank = rank.max(role.inner.position as u64);
            } else {
                warn!(user_id = ?user_id, role_id = ?role_id, "user has role that doesnt exist");
            }
        }

        rank
    }
}
