//! permission calculator

// TODO: move this logic to rooms service

use common::v1::types::util::Time;
use common::v1::types::{
    Channel, ChannelType, Permission, PermissionOverwriteType, RoleId, RoomId, RoomMember,
    SERVER_USER_ID, UserId,
};
use lamprey_backend_core::types::permission::Permissions2Metadata;
use lamprey_backend_core::types::permission::{
    CheckVisibility, MemberState, PermissionBits, Permissions2, ResourceContext,
};
use tracing::warn;

use crate::prelude::*;
use crate::services::cache::CachedRoomMember;
use crate::services::rooms::{CachedChannel, LoadedRoom, RoomSnapshot};

// TODO: add a permission calculator for a dm/gdm channel?
// PERF: convert channel overwrites and role perms into bits; cache
// RoomPermissions in CachedRoom and recalculate it when a relevant sync message is received
// make sure to keep in sync with crate-sdk/src/cache/permissions.rs!
/// a permission calculator for a room
pub struct RoomPermissions<'a> {
    room: &'a LoadedRoom,
}

impl<'a> RoomPermissions<'a> {
    pub fn new(room: &'a LoadedRoom) -> Self {
        Self { room }
    }

    pub fn query(
        &self,
        user_id: Option<UserId>,
        channel: Option<&CachedChannel>,
    ) -> Permissions2<CheckVisibility> {
        let member = user_id.and_then(|uid| self.room.members.get(&uid).map(|m| &m.member));

        let mut bits = PermissionBits::default();
        let mut rank = 0u16;
        let mut channel_locked = false;
        let mut timed_out = false;
        let mut quarantined = false;

        // calculate base perms (includes mute/deafen)
        self.calculate_room_permissions(
            &mut bits,
            &mut rank,
            &mut timed_out,
            &mut quarantined,
            user_id,
            member,
        );

        // calculate channel overwrites
        // admins have full permissions
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
                if channel.inner.ty == ChannelType::ThreadPrivate {
                    if !bits.has(Permission::ThreadManage) && !bits.has(Permission::Admin) {
                        let is_member = if let Some(threads) = &self.room.threads {
                            user_id.is_some_and(|uid| {
                                threads
                                    .get(&channel.inner.id)
                                    .map_or(false, |t| t.members.contains_key(&uid))
                            })
                        } else {
                            // TODO: fetch thread from db
                            false
                        };
                        if !is_member {
                            return self.build_permissions2(
                                PermissionBits::default(),
                                rank,
                                Some(channel),
                                channel_locked,
                                None,
                                false,
                                false,
                            );
                        }
                    }
                }
            }
        }

        // mask perms for non-members, even if we have Admin
        if member.is_none() {
            if channel.is_some_and(|c| c.inner.ty == ChannelType::Broadcast) {
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

        self.build_permissions2(
            bits,
            rank,
            channel,
            channel_locked,
            member,
            timed_out,
            quarantined,
        )
    }

    fn build_permissions2(
        &self,
        bits: PermissionBits,
        rank: u16,
        channel: Option<&CachedChannel>,
        channel_locked: bool,
        member: Option<&RoomMember>,
        timed_out: bool,
        quarantined: bool,
    ) -> Permissions2<CheckVisibility> {
        let room_id = self.room.room.id;
        let context = match channel {
            Some(ch) if ch.inner.ty.is_thread() => {
                ResourceContext::Thread(Some(room_id), ch.inner.parent_id.unwrap(), ch.inner.id)
            }
            Some(ch) => ResourceContext::Channel(Some(room_id), ch.inner.id),
            None => ResourceContext::Room(room_id),
        };

        // NOTE: is this logic correct?
        let visible = match channel {
            Some(_) => bits.has(Permission::Admin) || bits.has(Permission::ChannelView),
            None => self.room.room.public || member.is_some(),
        };

        let member_state = match member {
            None => MemberState::Lurker,
            Some(m) => MemberState::Joined {
                muted: m.mute,
                deafened: m.deaf,
                timed_out,
                quarantined: m.quarantined,
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

    fn calculate_room_permissions(
        &self,
        bits: &mut PermissionBits,
        rank: &mut u16,
        timed_out: &mut bool,
        quarantined: &mut bool,
        user_id: Option<UserId>,
        member: Option<&RoomMember>,
    ) {
        // the server user and room owner get full permissions
        if user_id.is_some_and(|uid| uid == SERVER_USER_ID || self.room.room.owner_id == Some(uid))
        {
            *rank = u16::MAX;
            *bits = Permission::Admin.into();
            return;
        }

        let Some(member) = member else {
            if self.room.room.public {
                let everyone_role_id: RoleId = self.room.room.id.into_inner().into();
                if let Some(role) = self.room.roles.get(&everyone_role_id) {
                    bits.add_all(role.allow);
                    bits.remove_all(role.deny);
                }
            }
            return;
        };

        let mut allowed_bits = PermissionBits::default();
        let mut denied_bits = PermissionBits::default();
        let everyone_role_id: RoleId = self.room.room.id.into_inner().into();

        // NOTE: the everyone role should always exist
        if let Some(role) = self.room.roles.get(&everyone_role_id) {
            allowed_bits.add_all(role.allow);
            denied_bits.add_all(role.deny);
        }

        for role_id in &member.roles {
            if let Some(role) = self.room.roles.get(role_id) {
                allowed_bits.add_all(role.allow);
                denied_bits.add_all(role.deny);
                *rank = (*rank).max(role.inner.position);
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
        cc: &CachedChannel,
        member: Option<&RoomMember>,
    ) {
        if let Some(parent_id) = cc.inner.parent_id {
            if let Some(parent) = self.room.channels.get(&parent_id) {
                self.calculate_channel_permissions(bits, channel_locked, timed_out, parent, member);
            }
        }

        self.apply_channel_overwrites(bits, cc, member);
        self.apply_channel_locked(bits, channel_locked, timed_out, cc, member, bits);
    }

    fn apply_channel_overwrites(
        &self,
        bits: &mut PermissionBits,
        cc: &CachedChannel,
        member: Option<&RoomMember>,
    ) {
        if cc.overwrites.is_empty() {
            return;
        }

        let everyone_id = self.room.room.id.into_inner().into();
        if let Some(ow) = cc.overwrites.get(&everyone_id) {
            bits.add_all(ow.allow);
            bits.remove_all(ow.deny);
        }

        if let Some(member) = member {
            for role_id in &member.roles {
                if let Some(ow) = cc.overwrites.get(&role_id.into_inner().into()) {
                    if ow.ty == PermissionOverwriteType::Role {
                        bits.add_all(ow.allow);
                        bits.remove_all(ow.deny);
                    }
                }
            }

            // NOTE: should i still apply overwrites even for non members?
            if let Some(ow) = cc.overwrites.get(&member.user_id.into_inner().into()) {
                if ow.ty == PermissionOverwriteType::User {
                    bits.add_all(ow.allow);
                    bits.remove_all(ow.deny);
                }
            }
        }
    }

    fn apply_channel_locked(
        &self,
        bits: &PermissionBits,
        channel_locked: &mut bool,
        timed_out: &mut bool,
        cc: &CachedChannel,
        member: Option<&RoomMember>,
        computed_bits: &PermissionBits,
    ) {
        let Some(locked) = &cc.inner.locked else {
            return;
        };
        let is_expired = locked.until.is_some_and(|until| until <= Time::now_utc());
        if is_expired {
            return;
        }

        let has_bypass = member.map_or(false, |m| {
            m.roles
                .iter()
                .any(|r| locked.allow_roles.contains(&(*r).into()))
        });
        let has_perm = computed_bits.has(Permission::Admin)
            || computed_bits.has(Permission::ChannelManage)
            || (cc.inner.ty.is_thread() && computed_bits.has(Permission::ThreadManage));

        if !has_bypass && !has_perm {
            *timed_out = true;
            *channel_locked = true;
        }
    }
}
