// TODO: always keep server room loaded? that would keep EVERY member and user on the server in memory though, so maybe not?
// TODO: automatically shutdown idle rooms

use std::{collections::HashMap, sync::Arc};

use common::{
    v1::types::{Channel, MessageSync, Role, Room, RoomFeature, RoomMember, ThreadMember, User},
    v2::types::{ChannelId, RoleId, RoomId, UserId},
};
use lamprey_backend_core::types::permission::PermissionBits;
use tokio::sync::{mpsc, watch};

use crate::{
    prelude::*,
    services::rooms::{
        member_lists::{List, ListTarget},
        utils::{UnavailableReason, sync_room_id},
    },
};

pub mod member_lists;
pub mod utils;

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }
}

/// a handle for interacting with a room actor
#[derive(Clone)]
pub struct RoomHandle {
    snapshot: watch::Receiver<Arc<RoomSnapshot>>,
    // tx: watch::Sender<Arc<RoomSnapshot>>,
}

pub struct RoomSnapshot {
    state: RoomSnapshotState,
    kind: RoomHandleKind,
}

impl RoomSnapshot {
    // pub fn is_local(&self) -> bool { todo!() }
    // pub fn is_federated(&self) -> bool { todo!() }
}

// TODO: remove?
pub enum RoomHandleKind {
    /// a room that exists on this node
    Local,

    /// a room that exists on this server but not this node
    Remote,

    /// a room that exists on a remote server
    // NOTE: you can already tell if its federated based on room.remote
    Federated,
}

/// a member list on a remote node or federated server
// NOTE: does this work with both or should i split the struct? or should i merge this with List?
// should i create a trait MemberList for these?
#[derive(Debug)]
pub struct ProxiedList {
    // TODO
}

impl RoomData {
    fn handle_sync(&mut self, sync: &MessageSync) -> Result<()> {
        let Some(room_id) = sync_room_id(sync) else {
            return Ok(());
        };

        if room_id != self.room.id {
            return Ok(());
        };

        todo!()
    }

    // TODO: periodically cleanup idle member lists

    // pub fn ensure_sudo_if_needed(&self, auth: &Auth) -> Result<()> {}
    // pub fn ensure_mfa_if_needed(&self, auth: &Auth) -> Result<()> {}
    pub fn ensure_feature(&self, _feature: &RoomFeature) -> Result<()> {
        todo!()
    }

    pub fn get_channel(&self, _channel_id: ChannelId) -> Option<&CachedChannel> {
        todo!()
    }

    pub fn get_role(&self, _role_id: &RoleId) -> Option<&CachedRole> {
        todo!()
    }

    pub fn get_member(&self, _user_id: UserId) -> Option<&CachedRoomMember> {
        todo!()
    }

    /// query permissions
    ///
    /// - passing in `channel` will calculate permissions in that channel
    /// - using `None` for `member` will calculate the default permissions (public room defaults)
    pub fn permissions(
        &self,
        _member: Option<&RoomMember>,
        _channel: Option<&Channel>,
        // ) -> Result<Permissions> {
    ) -> Result<()> {
        todo!()
    }
}
