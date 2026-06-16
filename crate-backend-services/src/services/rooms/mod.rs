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

pub struct Service {
    handles: HashMap<RoomId, RoomHandle>,
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    /// get a handle to a room
    pub fn load(&self, _room_id: RoomId) -> RoomHandle {
        // immediately return room handle, spawn background task to load room data then members
        todo!()
    }

    // pub fn unload(&self, room_id: RoomId);
    // pub fn reload(&self, room_id: RoomId);

    // pub fn create(&self, ...) -> RoomHandle
    // pub fn edit(&self, ...) // maybe put this on RoomHandle?
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

pub enum RoomSnapshotState {
    Loading,
    Loaded(Arc<RoomData>),
    Unavailable(UnavailableReason),
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

#[derive(Debug)]
pub struct RoomData {
    pub room: Box<Room>,

    /// all channels in this room
    pub channels: HashMap<ChannelId, CachedChannel>,

    /// all roles in this room
    pub roles: HashMap<RoleId, CachedRole>,

    /// all loaded/active threads in this room
    ///
    /// may be None if threads are still loading
    pub threads: Option<HashMap<ChannelId, CachedChannel>>,

    /// the room's members
    pub members: RoomMembers,
    // NOTE: i could move documents, flumes, etc here? though documents cant exist outside of a room, but flumes can...
}

#[derive(Debug)]
pub enum RoomMembers {
    /// all room members are cached on the local server
    Cached {
        members: HashMap<UserId, CachedRoomMember>,
        member_lists: HashMap<ListTarget, List>,
    },

    /// currently proxying room member requests to either a remote node or a federated server
    Proxied {
        member_lists: HashMap<ListTarget, ProxiedList>,
    },

    /// members are currently loading
    Loading,

    /// this is a server room, so members will only be loaded as needed
    Server {
        members: HashMap<UserId, CachedRoomMember>,
    },
}

impl RoomMembers {
    /// returns whether this room's members are fully loaded
    pub fn is_fully_loaded(&self) -> bool {
        todo!()
    }
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

#[derive(Debug, Clone)]
pub struct CachedRoomMember {
    /// the room member
    pub member: RoomMember,

    /// the user associated with the room member
    pub user: Arc<User>,
}

// TODO: use ChannelItem from crate-backend-services/src/channels.rs?
#[derive(Clone, Debug)]
pub struct CachedChannel {
    /// the channel itself
    pub inner: Channel,
}

#[derive(Clone, Debug)]
pub struct CachedRole {
    /// the role itself
    pub inner: Role,

    /// allowed permissions as a bitfield
    pub allow: PermissionBits,

    /// denied permissions as a bitfield
    pub deny: PermissionBits,
}

impl From<Role> for CachedRole {
    fn from(value: Role) -> Self {
        todo!()
    }
}

impl From<Channel> for CachedChannel {
    fn from(value: Channel) -> Self {
        todo!()
    }
}

pub enum RoomEvent {
    /// a sync event happened in this room
    Sync(MessageSync),

    /// the room's snapshot changed
    Update(RoomSnapshot),

    /// room was unloaded
    ///
    /// this is not emitted if the room is being reloaded. instead, the room snapshot state will become `Loading`
    Unload,
}

impl RoomHandle {
    /// wait until the room has successfully loaded
    ///
    /// - `with_members` will wait until all room members are loaded
    /// - `fail_if_unavailable` returns an error if the room is or becomes unavailable
    pub async fn ready(
        &mut self,
        with_members: bool,
        fail_if_unavailable: bool,
    ) -> Result<Arc<RoomData>> {
        let s = self
            .snapshot
            .wait_for(|s| match &s.state {
                RoomSnapshotState::Loading => false,
                // RoomSnapshotState::Loaded(data) => !with_members || data.members_loaded,
                RoomSnapshotState::Loaded(data) => todo!(),
                RoomSnapshotState::Unavailable(r) => r.is_fatal() || fail_if_unavailable,
            })
            .await
            .expect("todo better error handling");
        let data = match &s.state {
            RoomSnapshotState::Loaded(data) => Arc::clone(data),
            RoomSnapshotState::Unavailable(_) => todo!("return err"),
            _ => unreachable!(),
        };
        Ok(data)
    }

    /// get the current room snapshot
    pub fn snapshot(&self) -> Arc<RoomSnapshot> {
        Arc::clone(&self.snapshot.borrow())
    }

    /// get the current room data
    pub fn data(&self) -> Result<Arc<RoomData>> {
        match &self.snapshot.borrow().state {
            RoomSnapshotState::Loading => Err(Error::BadStatic("room is still loading")),
            RoomSnapshotState::Loaded(_) => todo!(),
            RoomSnapshotState::Unavailable(_) => Err(Error::BadStatic("room is unavailable")),
        }
    }

    pub fn subscribe(&self) -> mpsc::Receiver<Arc<RoomEvent>> {
        todo!()
    }

    // pub fn reload(&self) {
    //     todo!()
    // }

    // pub fn unload(&self) {
    //     todo!()
    // }

    // /// create a subscription to a member list
    // pub fn member_list(&self, conn_id: ConnectionId) -> MemberList {
    //     todo!()
    // }

    // pub fn handle_sync(&self, sync: MessageSync) {
    //     todo!()
    // }
}
