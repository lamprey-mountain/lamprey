//! minimal "summary" types
//!
//! these types contain the minimum amount of information required for the server to work, since lots of these will be retained in memory
// TODO: also use these for permissions too?

use std::sync::Arc;

use common::v1::types::{util::Time, Permission, Role, RoleId, Room, RoomSecurity, UserId};

pub struct RoomMemberSummary {
    // required for member lists
    pub user_name: Arc<str>,
    pub override_name: Option<Arc<str>>,

    // required for permissions
    pub roles: Vec<RoleId>,
    pub mute: bool,
    pub deaf: bool,
    pub timeout_until: Option<Time>,
}

pub struct ThreadMemberSummary {
    // nothing needed here?
}

pub struct RoomSummary {
    // required for permissions
    pub owner_id: Option<UserId>,
    pub security: RoomSecurity,
}

pub struct RoleSummary {
    // required for member lists
    pub position: u64,
    pub hoist: bool,

    // required for permissions
    pub allow: PermissionBits,
    pub deny: PermissionBits,
}

/// bitflags to represent permissions
#[rustfmt::skip]
pub enum PermissionBits {
    Admin              = 1 << 0,
    IntegrationsManage = 1 << 1,
    EmojiManage        = 1 << 2,
    // etc...
}

impl From<Permission> for PermissionBits {
    fn from(value: Permission) -> Self {
        todo!()
    }
}

impl From<&[Permission]> for PermissionBits {
    fn from(value: &[Permission]) -> Self {
        todo!()
    }
}

impl From<&Room> for RoomSummary {
    fn from(value: &Room) -> Self {
        todo!()
    }
}

impl From<&Role> for RoleSummary {
    fn from(value: &Role) -> Self {
        todo!()
    }
}

impl RoomMemberSummary {
    pub fn name(&self) -> Arc<str> {
        Arc::clone(self.override_name.as_ref().unwrap_or(&self.user_name))
    }
}
