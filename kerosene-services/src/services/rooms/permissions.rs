use common::{
    v1::types::{Channel, ChannelType, Permission, RoomMember, util::Time},
    v2::types::{ChannelId, RoleId, UserId},
};
use im::HashMap;
use kerosene_core::types::permission::{
    BROADCAST_LURKER_PERMS, CheckVisibility, PermissionBits, Permissions2, QUARANTINE_PERMS,
    VIEW_PERMS,
};

// TODO: finish implementing

/// a permission calculator for a room
// PERF: use slotmap
pub struct RoomPermissionsCalculator {
    everyone: PermissionBitset,
    roles: HashMap<RoleId, PermissionRole>,
    overwrites: HashMap<ChannelId, Overwrites>,
}

struct PermissionRole {
    /// the effective position of this role
    ///
    /// `role.position` tiebroken by `role.id`
    resolved_position: u64,

    set: PermissionBitset,
}

/// permission overwrites for a channel
struct Overwrites {
    everyone: PermissionBitset,
    roles: HashMap<RoleId, PermissionBitset>,
    users: HashMap<UserId, PermissionBitset>,
}

impl Overwrites {
    pub fn from_channel(chan: &Channel) {
        todo!()
    }
}

/// allowed/denide permissions as bitfields
#[derive(Clone, Copy, Default, Debug)]
struct PermissionBitset {
    allow: PermissionBits,
    deny: PermissionBits,
}

impl PermissionBitset {
    pub fn empty() -> Self {
        Self::default()
    }

    /// calculate the allowed permissions
    #[inline]
    pub fn resolved(&self) -> PermissionBits {
        self.allow - self.deny
    }

    #[inline]
    pub fn layer(self, other: Self) -> Self {
        Self {
            allow: (self.resolved() | other.allow) - other.deny,
            deny: PermissionBits::default(),
        }
    }
}

impl std::ops::Add for PermissionBitset {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            allow: self.allow | rhs.allow,
            deny: self.deny | rhs.deny,
        }
    }
}

impl std::ops::AddAssign for PermissionBitset {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

// pub struct RoomPermissions {
//     pub visible: bool,
//     pub rank: u64,
//     pub permission_bits: PermissionBits,
// }

impl RoomPermissionsCalculator {
    /// query permissions for a user
    ///
    /// - passing in `channel` will calculate permissions in that channel
    /// - using `None` for user_id will calculate the default permissions (public room defaults)
    pub fn query(
        &self,
        member: Option<&RoomMember>,
        channel: Option<&Channel>,
    ) -> Permissions2<CheckVisibility> {
        let (mut set, rank) = self.calculate_room(member);

        if let Some(channel) = channel {
            set = set.layer(self.calculate_channel(member, channel));
        }

        todo!()
    }

    /// calculate base permissions for a member in a room
    fn calculate_room(&self, member: Option<&RoomMember>) -> (PermissionBitset, u64) {
        let set = self.everyone;

        let mut role_set = PermissionBitset::empty();
        let mut rank = 0;

        if let Some(member) = member {
            for role_id in &member.roles {
                if let Some(role) = self.roles.get(role_id) {
                    role_set += role.set;
                    rank = rank.max(role.resolved_position);
                }
            }
        }

        (set.layer(role_set), rank)
    }

    /// calculate permissions for a channel
    fn calculate_channel(
        &self,
        member: Option<&RoomMember>,
        channel: &Channel,
    ) -> PermissionBitset {
        let Some(overwrites) = self.overwrites.get(&channel.id) else {
            return PermissionBitset::empty();
        };

        let mut set = overwrites.everyone;

        if let Some(member) = member {
            let mut role_set = PermissionBitset::empty();
            for role_id in &member.roles {
                if let Some(o) = overwrites.roles.get(role_id) {
                    role_set += *o;
                }
            }
            set = set.layer(role_set);

            if let Some(o) = overwrites.users.get(&member.user_id) {
                set = set.layer(*o);
            }
        }

        set
    }

    /// mask permissions
    fn apply_mask(
        &self,
        member: Option<&RoomMember>,
        channel: Option<&Channel>,
        bits: &mut PermissionBits,
    ) {
        match member {
            Some(member) => {
                if member.quarantined && !bits.has(Permission::Admin) {
                    bits.mask(QUARANTINE_PERMS);
                }

                if let Some(timeout_until) = member.timeout_until {
                    if timeout_until > Time::now_utc() {
                        bits.mask(VIEW_PERMS);
                    }
                }
            }
            None => {
                if channel.is_some_and(|c| c.ty == ChannelType::Broadcast) {
                    bits.mask(BROADCAST_LURKER_PERMS);
                } else {
                    bits.mask(VIEW_PERMS);
                }
            }
        }
    }
}
