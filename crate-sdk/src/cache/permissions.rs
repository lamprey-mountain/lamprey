use common::v1::types::util::Time;
use common::v1::types::{
    Channel, ChannelType, Permission, PermissionBits, PermissionOverwriteType, RoleId, RoomMember,
    SERVER_USER_ID, UserId,
};
use lamprey_backend_core::types::permission::{
    CheckVisibility, MemberState, Permissions2, Permissions2Metadata, ResourceContext,
};
use tracing::warn;

use crate::cache::CachedRoom;

impl CachedRoom {
    /// get a permission calculator for this room
    pub fn permissions(&self) -> RoomPermissions<'_> {
        RoomPermissions { room: self }
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
    // bits: PermissionBits,
    // visible: bool,
    // rank: u64,
    // channel_locked, timed_out, quarantined, etc: bool,
}

impl Permissions {
    // /// Check if a specific permission is granted (Admins have all permissions)
    // pub fn has(&self, perm: Permission) -> bool {
    //     self.bits.has(Permission::Admin) || self.bits.has(perm)
    // }

    // pub fn visible(&self) -> bool {
    //     self.visible
    // }

    // pub fn rank(&self) -> u64 {
    //     self.rank
    // }
}

// FIXME: handle slowmode for message, thread

impl RoomPermissions<'_> {
    // TODO: use this?
    // pub fn query(&self, member: Option<&RoomMember>, channel: Option<&Channel>) -> Permissions {
    //     todo!()
    // }

    /// query permissions for a user
    ///
    /// - passing in `channel` will calculate permissions in that channel
    /// - using `None` for user_id will calculate the default permissions (public room defaults)
    // TODO(?): use a better (sdk-specific?) type instead of Permissions2
    // TODO: use CachedChannel
    pub fn query(
        &self,
        user_id: Option<UserId>,
        channel: Option<&Channel>,
    ) -> Permissions2<CheckVisibility> {
        let member = user_id.and_then(|uid| self.room.members.get(&uid));

        let mut bits = PermissionBits::default();
        let mut rank = 0u16;
        let mut channel_locked = false;
        let mut timed_out = false;
        let mut quarantined = false;

        if !self.room.inner.public && member.is_none() {
            return self.build_permissions2(
                bits,
                rank,
                channel,
                channel_locked,
                MemberState::Lurker,
            );
        }

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
                self.calculate_channel_permissions(
                    &mut bits,
                    &mut channel_locked,
                    &mut timed_out,
                    channel,
                    member,
                );

                // private thread logic
                if channel.ty == ChannelType::ThreadPrivate {
                    if !bits.has(Permission::ThreadManage) {
                        let is_member = user_id.is_some_and(|uid| {
                            self.room
                                .channels
                                .get(&channel.id)
                                .map_or(false, |t| t.members.contains_key(&uid))
                        });

                        if !is_member {
                            return self.build_permissions2(
                                PermissionBits::default(),
                                rank,
                                Some(channel),
                                channel_locked,
                                MemberState::Lurker,
                            );
                        }
                    }
                }
            }
        }

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

        let member_state = match member {
            None => MemberState::Lurker,
            Some(m) => MemberState::Joined {
                muted: m.mute,
                deafened: m.deaf,
                timed_out,
                quarantined: m.quarantined,
            },
        };

        self.build_permissions2(bits, rank, channel, channel_locked, member_state)
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
        channel_locked: &mut bool,
        timed_out: &mut bool,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        if let Some(parent_id) = channel.parent_id {
            if let Some(parent_cc) = self.room.channels.get(&parent_id) {
                self.calculate_channel_permissions(
                    bits,
                    channel_locked,
                    timed_out,
                    &parent_cc.inner,
                    member,
                );
            } else {
                warn!(
                    channel_id = ?channel.id,
                    parent_id = ?parent_id,
                    "channel has a parent_id that doesn't exist"
                );
            }
        }

        self.apply_channel_locked(bits, channel_locked, timed_out, channel, member);
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

    fn apply_channel_locked(
        &self,
        bits: &PermissionBits,
        channel_locked: &mut bool,
        timed_out: &mut bool,
        channel: &Channel,
        member: Option<&RoomMember>,
    ) {
        // handle locked channels/threads
        if let Some(locked) = &channel.locked {
            let is_expired = locked.until.is_some_and(|until| until <= Time::now_utc());
            if !is_expired {
                *channel_locked = true;

                // the member has a role that is explicitly allowed by the lock
                let has_bypass = member.map_or(false, |m| {
                    m.roles
                        .iter()
                        .any(|r| locked.allow_roles.contains(&(*r).into()))
                });

                // or the member has the Manage Channels permission
                // or this is a thread and the member has the Manage Threads permission
                let has_perm = bits.has(Permission::ChannelManage)
                    || (channel.ty.is_thread() && bits.has(Permission::ThreadManage));

                if !has_bypass && !has_perm {
                    *timed_out = true;
                }
            }
        }
    }

    fn build_permissions2(
        &self,
        bits: PermissionBits,
        rank: u16,
        channel: Option<&Channel>,
        channel_locked: bool,
        member_state: MemberState,
    ) -> Permissions2<CheckVisibility> {
        let room_id = self.room.inner.id;
        let context = match channel {
            Some(ch) if ch.is_thread() => ResourceContext::Thread(
                Some(room_id),
                ch.parent_id.unwrap_or(room_id.into_inner().into()),
                ch.id,
            ),
            Some(ch) => ResourceContext::Channel(Some(room_id), ch.id),
            None => ResourceContext::Room(room_id),
        };

        let visible = match channel {
            Some(_) => bits.has(Permission::Admin) || bits.has(Permission::ChannelView),
            None => match member_state {
                MemberState::Lurker => self.room.inner.public,
                MemberState::Joined { .. } => true,
            },
        };

        Permissions2 {
            visible,
            context,
            bits,
            metadata: Permissions2Metadata {
                rank,
                member_state,
                channel_locked,
                channel_slowmode_thread_active: false,
                channel_slowmode_message_active: false,
            },
            state: CheckVisibility,
        }
    }

    /// get whether a user (or guest) can view this room
    pub fn can_view_room(&self, user_id: Option<UserId>) -> bool {
        let is_public = self.room.inner.public;
        if is_public {
            // anyone can view public rooms
            true
        } else if let Some(user_id) = user_id {
            // you can view private rooms you're a member of
            self.room.members.contains_key(&user_id)
        } else {
            // otherwise, deny
            false
        }
    }

    /// get the rank of a user
    ///
    /// a user's rank is their highest role's position
    pub fn rank(&self, user_id: UserId) -> u64 {
        if self.room.inner.owner_id == Some(user_id) {
            return u64::MAX;
        }

        let Some(member) = self.room.members.get(&user_id) else {
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
