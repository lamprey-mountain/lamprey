//! permission calculator

use std::sync::Arc;

use common::v1::types::util::Time;
use common::v1::types::{
    Channel, ChannelType, Permission, PermissionOverwriteType, RoleId, RoomId, RoomMember, UserId,
    SERVER_USER_ID,
};
use lamprey_backend_core::types::permission::{
    CheckVisibility, MemberState, PermissionBits, Permissions2, ResourceContext,
    BROADCAST_LURKER_PERMS, QUARANTINE_PERMS, VIEW_PERMS,
};
use lamprey_backend_data_postgres::Permissions2Metadata;
use tracing::warn;

use crate::{
    services::rooms::{CachedChannel, RoomSnapshot},
    Error, Result,
};

/// a permission calculator for a room
pub struct PermissionsCalculator {
    pub room_id: RoomId,
    pub owner_id: Option<UserId>,
    pub public: bool,
    pub room: Arc<RoomSnapshot>,
}

impl PermissionsCalculator {
    /// query permissions for a room member, optionally in a specific channel
    pub fn query(
        &self,
        user_id: UserId,
        channel: Option<&Channel>,
    ) -> Result<Permissions2<CheckVisibility>> {
        self.query2(Some(user_id), channel)
    }

    /// query permissions for a user
    ///
    /// - passing in `channel` will calculate permissions in that channel
    /// - using `None` for user_id will calculate the default permissions (public room defaults)
    pub fn query2(
        &self,
        user_id: Option<UserId>,
        channel: Option<&Channel>,
    ) -> Result<Permissions2<CheckVisibility>> {
        let Some(user_id) = user_id else {
            // calculate default room permissions for lurkers/unauthed sessions

            let Some(data) = self.room.get_data() else {
                return Err(Error::ServiceUnavailable);
            };

            let mut bits = PermissionBits::default();
            let mut channel_locked = false;

            if self.public {
                // use default perms (everyone role)
                let everyone_role_id: RoleId = self.room_id.into_inner().into();

                if let Some(role) = data.roles.get(&everyone_role_id) {
                    bits.add_all(role.allow);
                    bits.remove_all(role.deny);
                }

                if let Some(channel) = channel {
                    if let Some(cached_channel) = data.channels.get(&channel.id) {
                        self.apply_channel_overwrites(
                            &mut bits,
                            &mut channel_locked,
                            &mut false,
                            cached_channel,
                            None,
                        );
                    }
                }
            }

            let context = match channel {
                Some(ch) if ch.is_thread() => {
                    ResourceContext::Thread(Some(self.room_id), ch.parent_id.unwrap(), ch.id)
                }
                Some(ch) => ResourceContext::Channel(Some(self.room_id), ch.id),
                None => ResourceContext::Room(self.room_id),
            };

            if channel.is_some_and(|c| c.ty == ChannelType::Broadcast) {
                bits.mask(BROADCAST_LURKER_PERMS);
            } else {
                bits.mask(VIEW_PERMS);
            }

            let perms = Permissions2 {
                visible: self.public,
                context,
                bits,
                metadata: Permissions2Metadata {
                    rank: 0,
                    member_state: MemberState::Lurker,
                    channel_locked,
                    channel_slowmode_thread_active: false,
                    channel_slowmode_message_active: false,
                },
                state: CheckVisibility,
            };

            return Ok(perms);
        };

        self.query_inner(user_id, channel)
    }

    /// get whether a user (or guest) can view this room
    pub fn can_view_room(&self, user_id: Option<UserId>) -> bool {
        let is_public = self.room.get_data().is_some_and(|d| d.room.public);
        if is_public {
            // anyone can view public rooms
            true
        } else if let Some(user_id) = user_id {
            // you can view private rooms you're a member of
            self.room.get_member(&user_id).is_some()
        } else {
            // otherwise, deny
            false
        }
    }

    fn query_inner(
        &self,
        user_id: UserId,
        channel: Option<&Channel>,
    ) -> Result<Permissions2<CheckVisibility>> {
        let Some(data) = self.room.get_data() else {
            return Err(Error::ServiceUnavailable);
        };

        let member = data.members.get(&user_id).map(|m| &m.member);

        let mut bits = PermissionBits::default();
        let mut rank = 0u16;
        let mut channel_locked = false;
        let mut timed_out = false;
        let mut quarantined = false;

        if !data.room.public && member.is_none() {
            // non-member in private room - lurker with no perms
            return Ok(self.build_permissions2(
                bits,
                rank,
                channel,
                channel_locked,
                MemberState::Lurker,
            ));
        }

        // calculate base perms (includes mute/deafen)
        self.calculate_room_permissions2(
            &mut bits,
            &mut rank,
            &mut timed_out,
            &mut quarantined,
            user_id,
            member,
        )?;

        // admins have full permissions
        if !bits.has(Permission::Admin) {
            if let Some(channel) = channel {
                // only calculate channel permissions if the channel exists in cache
                // (channels not in cache have no overwrites)
                if let Some(cached_channel) = data.channels.get(&channel.id) {
                    self.calculate_channel_permissions2(
                        &mut bits,
                        &mut channel_locked,
                        &mut timed_out,
                        cached_channel,
                        member,
                    );
                }
            }
        }

        // mask permissions for lurkers/non-members
        if member.is_none() {
            if channel.is_some_and(|c| c.ty == ChannelType::Broadcast) {
                bits.mask(BROADCAST_LURKER_PERMS);
            } else {
                bits.mask(VIEW_PERMS);
            }
        }

        if quarantined && !bits.has(Permission::Admin) {
            bits.mask(QUARANTINE_PERMS);
        }

        if timed_out {
            bits.mask(VIEW_PERMS);
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

        Ok(self.build_permissions2(bits, rank, channel, channel_locked, member_state))
    }

    fn build_permissions2(
        &self,
        bits: PermissionBits,
        rank: u16,
        channel: Option<&Channel>,
        channel_locked: bool,
        member_state: MemberState,
    ) -> Permissions2<CheckVisibility> {
        let context = match channel {
            Some(ch) if ch.is_thread() => {
                ResourceContext::Thread(Some(self.room_id), ch.parent_id.unwrap(), ch.id)
            }
            Some(ch) => ResourceContext::Channel(Some(self.room_id), ch.id),
            None => ResourceContext::Room(self.room_id),
        };

        let visible = match member_state {
            MemberState::Lurker => self.public,
            MemberState::Joined { .. } => true,
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

    /// calculate base permissions for a member in a room
    fn calculate_room_permissions2(
        &self,
        bits: &mut PermissionBits,
        rank: &mut u16,
        timed_out: &mut bool,
        quarantined: &mut bool,
        user_id: UserId,
        member: Option<&RoomMember>,
    ) -> Result<()> {
        // root user and owners have full permissions
        if user_id == SERVER_USER_ID || self.owner_id == Some(user_id) {
            *rank = u16::MAX;
            *bits = Permission::Admin.into();
            return Ok(());
        }

        let Some(data) = self.room.get_data() else {
            return Err(Error::ServiceUnavailable);
        };

        let Some(member) = member else {
            if self.public {
                // use public/default perms
                let everyone_role_id: RoleId = self.room_id.into_inner().into();

                if let Some(role) = data.roles.get(&everyone_role_id) {
                    bits.add_all(role.allow);
                    bits.remove_all(role.deny);
                }
            } else {
                // the member doesnt exist here and room not public; no perms
            }

            return Ok(());
        };

        // calculate role permissions using bit operations
        let mut allowed_bits = PermissionBits::default();
        let mut denied_bits = PermissionBits::default();

        let everyone_role_id = self.room_id.into_inner().into();

        for role in data.roles.values() {
            if role.inner.id == everyone_role_id || member.roles.contains(&role.inner.id) {
                allowed_bits.add_all(role.allow);
                denied_bits.add_all(role.deny);
                *rank = (*rank).max(role.inner.position as u16);
            }
        }

        bits.add_all(allowed_bits);

        // admins cannot have any permissions revoked
        if bits.has(Permission::Admin) {
            return Ok(());
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

        Ok(())
    }

    /// recursively calculate channel permissions
    fn calculate_channel_permissions2(
        &self,
        bits: &mut PermissionBits,
        channel_locked: &mut bool,
        timed_out: &mut bool,
        cc: &CachedChannel,
        member: Option<&RoomMember>,
    ) {
        if let Some(parent_id) = cc.inner.parent_id {
            if let Some(data) = self.room.get_data() {
                if let Some(parent) = data.channels.get(&parent_id) {
                    self.calculate_channel_permissions2(
                        bits,
                        channel_locked,
                        timed_out,
                        parent,
                        member,
                    );
                } else {
                    warn!(
                        channel_id = ?cc.inner.id,
                        parent_id = ?parent_id,
                        "channel has a parent_id that doesn't exist"
                    );
                }
            }
        }

        self.apply_channel_overwrites(bits, channel_locked, timed_out, cc, member);
    }

    /// apply the permission overwrites for a channel to a permissions set
    fn apply_channel_overwrites(
        &self,
        bits: &mut PermissionBits,
        channel_locked: &mut bool,
        timed_out: &mut bool,
        cc: &CachedChannel,
        member: Option<&RoomMember>,
    ) {
        // handle locked channels/threads
        if let Some(locked) = &cc.inner.locked {
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
                    || (cc.inner.ty.is_thread() && bits.has(Permission::ThreadManage));

                if !has_bypass && !has_perm {
                    *timed_out = true;
                }
            }
        }

        if cc.overwrites.is_empty() {
            return;
        }

        let everyone_id = self.room_id.into_inner();

        // 1. apply everyone allows
        if let Some(ow) = cc.overwrites.get(&everyone_id) {
            bits.add_all(ow.allow);
        }

        // 2. apply everyone denies
        if let Some(ow) = cc.overwrites.get(&everyone_id) {
            bits.remove_all(ow.deny);
        }

        let Some(member) = member else { return };

        // 3. apply role allows
        for role_id in &member.roles {
            if let Some(ow) = cc.overwrites.get(&role_id.into_inner()) {
                if ow.ty == PermissionOverwriteType::Role {
                    bits.add_all(ow.allow);
                }
            }
        }

        // 4. apply role denies
        for role_id in &member.roles {
            if let Some(ow) = cc.overwrites.get(&role_id.into_inner()) {
                if ow.ty == PermissionOverwriteType::Role {
                    bits.remove_all(ow.deny);
                }
            }
        }

        // 5. apply user allows
        if let Some(ow) = cc.overwrites.get(&member.user_id.into_inner()) {
            if ow.ty == PermissionOverwriteType::User {
                bits.add_all(ow.allow);
            }
        }

        // 6. apply user denies
        if let Some(ow) = cc.overwrites.get(&member.user_id.into_inner()) {
            if ow.ty == PermissionOverwriteType::User {
                bits.remove_all(ow.deny);
            }
        }
    }

    /// get the rank of this user, the position of the highest role this user has
    pub fn rank(&self, user_id: UserId) -> u64 {
        if self.owner_id == Some(user_id) {
            return u64::MAX;
        }

        let Some(data) = self.room.get_data() else {
            return 0;
        };

        let member = data.members.get(&user_id).map(|m| &m.member);
        let Some(member) = member else {
            // user is not a member, return 0
            return 0;
        };

        let mut rank = 0u64;
        for role_id in &member.roles {
            if let Some(role) = data.roles.get(role_id) {
                rank = rank.max(role.inner.position as u64);
            } else {
                warn!(user_id = ?user_id, role_id = ?role_id, "user has role that doesnt exist");
            }
        }

        rank
    }
}
