use common::v1::types::util::Time;
use common::v1::types::{
    Channel, ChannelType, Permission, PermissionBits, PermissionOverwriteType, RoleId, RoomMember,
    SERVER_USER_ID, UserId,
};
use tracing::warn;

use crate::cache::CachedRoom;

impl CachedRoom {
    /// get a permission calculator for this room
    pub fn permissions(&self) -> RoomPermissions<'_> {
        RoomPermissions::new(self)
    }
}

// TODO: add a permission calculator for a dm/gdm channel?
// PERF: convert channel overwrites and role perms into bits; cache
// RoomPermissions in CachedRoom and recalculate it when a relevant sync message is received
pub struct RoomPermissions<'a> {
    room: &'a CachedRoom,
}

#[derive(Debug, Clone)]
pub struct Permissions {
    bits: PermissionBits,
    visible: bool,
    rank: u16,
    // TODO(?): add channel_locked, timed_out, quarantined, etc fields
}

impl Permissions {
    /// Check if a specific permission is granted
    ///
    /// Admins have all permissions
    pub fn has(&self, perm: Permission) -> bool {
        self.bits.has(Permission::Admin) || self.bits.has(perm)
    }

    /// returns whether the user can view this resource
    pub fn visible(&self) -> bool {
        self.visible
    }

    // TODO: only provide inside rooms
    /// get the rank of a user
    ///
    /// a user's rank is their highest role's position
    pub fn rank(&self) -> u16 {
        self.rank
    }
}

// FIXME: handle slowmode for message, thread

impl<'a> RoomPermissions<'a> {
    /// create a new permission calculator
    pub fn new(room: &'a CachedRoom) -> Self {
        Self { room }
    }

    // TODO: pass RoomMember instead of UserId(?)
    // TODO: pass CachedChannel instead of Channel
    // pub fn query(&self, member: Option<&RoomMember>, channel: Option<&Channel>) -> Permissions {
    //     todo!()
    // }

    /// query permissions for a user
    ///
    /// - passing in `channel` will calculate permissions in that channel
    /// - using `None` for user_id will calculate the default permissions (public room defaults)
    pub fn query(&self, user_id: Option<UserId>, channel: Option<&Channel>) -> Permissions {
        let member = user_id.and_then(|uid| self.room.members.get(&uid));

        let mut bits = PermissionBits::default();
        let mut rank = 0u16;
        let mut timed_out = false;
        let mut quarantined = false;

        self.calculate_room_permissions(
            &mut bits,
            &mut rank,
            &mut timed_out,
            &mut quarantined,
            user_id,
            member,
        );

        if !bits.has(Permission::Admin) {
            if let Some(channel) = channel {
                self.calculate_channel_permissions(&mut bits, &mut timed_out, channel, member);

                // private thread logic
                if channel.ty == ChannelType::ThreadPrivate {
                    if !bits.has(Permission::ThreadManage) && !bits.has(Permission::Admin) {
                        let is_thread_member = user_id.is_some_and(|uid| {
                            self.room
                                .channels
                                .get(&channel.id)
                                .map_or(false, |t| t.members.contains_key(&uid))
                        });

                        if !is_thread_member {
                            bits = PermissionBits::default();
                        }
                    }
                }
            }
        }

        // mask perms for non-members, even if we have Admin
        if member.is_none() {
            if channel.is_some_and(|c| c.ty == ChannelType::Broadcast) {
                bits.mask(PermissionBits::BROADCAST_LURKER_PERMS);
            } else {
                bits.mask(PermissionBits::VIEW_PERMS);
            }
        }

        if quarantined && !bits.has(Permission::Admin) {
            bits.mask(PermissionBits::QUARANTINE_PERMS);
        }

        if timed_out {
            bits.mask(PermissionBits::VIEW_PERMS);
        }

        // NOTE: is this logic correct?
        let visible = match channel {
            Some(_) => bits.has(Permission::Admin) || bits.has(Permission::ChannelView),
            None => self.room.inner.public || member.is_some(),
        };

        Permissions {
            visible,
            bits,
            rank,
        }
    }

    fn calculate_room_permissions(
        &self,
        bits: &mut PermissionBits,
        rank: &mut u16,
        timed_out: &mut bool,
        quarantined: &mut bool,
        user_id: Option<UserId>,
        member: Option<&RoomMember>,
    ) {
        if user_id.is_some_and(|uid| uid == SERVER_USER_ID || self.room.inner.owner_id == Some(uid))
        {
            *rank = u16::MAX;
            *bits = Permission::Admin.into();
            return;
        }

        let Some(member) = member else {
            if self.room.inner.public {
                let everyone_role_id: RoleId = self.room.inner.id.into_inner().into();
                if let Some(role) = self.room.roles.get(&everyone_role_id) {
                    bits.add_all(PermissionBits::from(role.allow.as_slice()));
                    bits.remove_all(PermissionBits::from(role.deny.as_slice()));
                }
            }
            return;
        };

        let mut allowed_bits = PermissionBits::default();
        let mut denied_bits = PermissionBits::default();
        let everyone_role_id: RoleId = self.room.inner.id.into_inner().into();

        for role in self.room.roles.values() {
            if role.id == everyone_role_id || member.roles.contains(&role.id) {
                allowed_bits.add_all(PermissionBits::from(role.allow.as_slice()));
                denied_bits.add_all(PermissionBits::from(role.deny.as_slice()));
                *rank = (*rank).max(role.position as u16);
            }
        }

        bits.add_all(allowed_bits);
        if bits.has(Permission::Admin) {
            return;
        }
        bits.remove_all(denied_bits);

        // handle timeout
        if let Some(timeout_until) = member.timeout_until {
            if timeout_until > Time::now_utc() {
                *timed_out = true;
            }
        }

        // quarantined by automod
        if member.quarantined {
            *quarantined = true;
        }
    }

    fn calculate_channel_permissions(
        &self,
        bits: &mut PermissionBits,
        timed_out: &mut bool,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        if let Some(parent_id) = channel.parent_id {
            if let Some(parent_cc) = self.room.channels.get(&parent_id) {
                self.calculate_channel_permissions(bits, timed_out, &parent_cc.inner, member);
            } else {
                warn!(
                    channel_id = ?channel.id,
                    parent_id = ?parent_id,
                    "channel has a parent_id that doesn't exist"
                );
            }
        }

        self.apply_channel_locked(bits, timed_out, channel, member);
        self.apply_channel_overwrites(bits, channel, member);
    }

    fn apply_channel_overwrites(
        &self,
        bits: &mut PermissionBits,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        if channel.permission_overwrites.is_empty() {
            return;
        }

        let everyone_id = self.room.inner.id.into_inner().into();

        // 1. apply everyone allows
        if let Some(ow) = channel
            .permission_overwrites
            .iter()
            .find(|o| o.id == everyone_id)
        {
            bits.add_all(PermissionBits::from(ow.allow.as_slice()));
        }

        // 2. apply everyone denies
        if let Some(ow) = channel
            .permission_overwrites
            .iter()
            .find(|o| o.id == everyone_id)
        {
            bits.remove_all(PermissionBits::from(ow.deny.as_slice()));
        }

        let Some(member) = member else { return };

        // 3. apply role allows
        for role_id in &member.roles {
            if let Some(ow) = channel
                .permission_overwrites
                .iter()
                .find(|o| o.id == role_id.into_inner().into())
            {
                if ow.ty == PermissionOverwriteType::Role {
                    bits.add_all(PermissionBits::from(ow.allow.as_slice()));
                }
            }
        }

        // 4. apply role denies
        for role_id in &member.roles {
            if let Some(ow) = channel
                .permission_overwrites
                .iter()
                .find(|o| o.id == role_id.into_inner().into())
            {
                if ow.ty == PermissionOverwriteType::Role {
                    bits.remove_all(PermissionBits::from(ow.deny.as_slice()));
                }
            }
        }

        // 5. apply user allows
        if let Some(ow) = channel
            .permission_overwrites
            .iter()
            .find(|o| o.id == member.user_id.into_inner().into())
        {
            if ow.ty == PermissionOverwriteType::User {
                bits.add_all(PermissionBits::from(ow.allow.as_slice()));
            }
        }

        // 6. apply user denies
        if let Some(ow) = channel
            .permission_overwrites
            .iter()
            .find(|o| o.id == member.user_id.into_inner().into())
        {
            if ow.ty == PermissionOverwriteType::User {
                bits.remove_all(PermissionBits::from(ow.deny.as_slice()));
            }
        }
    }

    /// handle locked channels/threads
    fn apply_channel_locked(
        &self,
        bits: &PermissionBits,
        timed_out: &mut bool,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        let Some(locked) = &channel.locked else {
            return;
        };

        let is_expired = locked.until.is_some_and(|until| until <= Time::now_utc());
        if is_expired {
            return;
        }

        // the member has a role that is explicitly allowed by the lock
        let has_bypass = member.map_or(false, |m| {
            m.roles
                .iter()
                .any(|r| locked.allow_roles.contains(&(*r).into()))
        });

        // or the member has the Manage Channels permission
        // or this is a thread and the member has the Manage Threads permission
        let has_perm = bits.has(Permission::Admin)
            || bits.has(Permission::ChannelManage)
            || (channel.ty.is_thread() && bits.has(Permission::ThreadManage));

        if !has_bypass && !has_perm {
            *timed_out = true;
        }
    }
}
