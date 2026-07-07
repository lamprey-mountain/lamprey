// NOTE: do i even need this service? i could have permission methods for rooms and channels

use common::{
    v1::types::{Permission, RoomMember, oauth::Scope},
    v2::types::{ChannelId, RoomId, SERVER_ROOM_ID, UserId},
};
use lamprey_backend_core::types::permission::PermissionBits;

use crate::prelude::*;

pub struct Service {
    //
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    pub async fn calculate_server(
        &self,
        user_id: Option<UserId>,
    ) -> Result<Permissions<PermissionsRoom>> {
        self.calculate_room(SERVER_ROOM_ID, user_id).await
    }

    pub async fn calculate_room(
        &self,
        _room_id: RoomId,
        _user_id: Option<UserId>,
    ) -> Result<Permissions<PermissionsRoom>> {
        todo!()
    }

    pub async fn calculate_channel(
        &self,
        _channel_id: ChannelId,
        _user_id: Option<UserId>,
    ) -> Result<Permissions<PermissionsChannel>> {
        todo!()
    }
}

// TODO: merge with crate-backend-core/src/types/permission/mod.rs
/// a set of resolved permissions
pub struct Permissions<C> {
    visible: bool,
    bits: PermissionBits,
    context: C,
}

/// a set of permission checks that must pass
pub struct Requirements<C> {
    needs: PermissionBits,
    always_visible: bool,
    context: C,
}

// TODO: impl sealed trait for these
pub struct RequirementsRoom;

pub struct RequirementsChannel {
    always_unlocked: bool,
    slowmode_thread: bool,
    slowmode_message: bool,
}

pub struct PermissionsRoom {
    room_member: Box<RoomMember>,
    rank: u16,
}

pub struct PermissionsChannel;

impl<C> Permissions<C> {
    pub fn new() -> Self {
        todo!()
    }

    pub fn has(&self, p: Permission) -> bool {
        todo!()
    }
}

impl Permissions<PermissionsRoom> {
    pub fn rank(&self) -> u16 {
        todo!()
    }
}

// // PERF: use this
// pub struct RequirementsBits(u128);

// impl RequirementsBits {
//     /// assume the target resource is always visible
//     ///
//     /// this is for invites, where the user should be able to view the target room/channel even if they haven't joined yet
//     pub const ALWAYS_VISIBLE: u128 = 1 << 0;

//     /// assume the target channel is always unlocked
//     pub const ALWAYS_UNLOCKED: u128 = 1 << 1;

//     /// require the user to pass thread slowmode in the target channel
//     pub const SLOWMODE_THREAD: u128 = 1 << 2;

//     /// require the user to pass message slowmode in the target channel
//     pub const SLOWMODE_MESSAGE: u128 = 1 << 3;

//     pub fn from_permission_bits(bits: PermissionBits) -> Self {
//         Self(bits.into_bits() << 4)
//     }

//     pub fn into_permission_bits(self) -> PermissionBits {
//         PermissionBits::from_bits(self.0 >> 4)
//     }
// }
