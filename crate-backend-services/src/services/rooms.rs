use std::{collections::HashMap, sync::Arc};

use common::{
    v1::types::{Channel, MessageSync, Role, Room, RoomMember, ThreadMember, User},
    v2::types::{ChannelId, RoleId, RoomId, UserId},
};
use tokio::sync::{mpsc, watch};

use crate::{
    prelude::*,
    services::rooms::member_lists::{List, ListTarget},
};

pub mod member_lists;
pub mod members;

pub struct Service {
    // pub members: members::Service,
    // writers: Cache<RoomId, RoomWriter>,
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    /// get a handle to a room
    pub fn load(&self, _room_id: RoomId) -> RoomHandle {
        todo!()
    }

    // pub fn invalidate(&self, room_id: RoomId);
    // pub fn reload(&self, room_id: RoomId);

    // pub fn create(&self, ...)
    // pub fn edit(&self, ...)
}

/// a handle for interacting with a room actor
pub struct RoomHandle {
    snapshot: watch::Receiver<Arc<RoomSnapshot>>,
}

/// a handle for updating a room actor
struct RoomWriter {
    snapshot: watch::Sender<Arc<RoomSnapshot>>,
}

pub struct RoomSnapshot {
    state: RoomSnapshotState,
}

pub enum RoomSnapshotState {
    Loading,
    Loaded(Arc<RoomData>),
    Unavailable(UnavailableReason),
}

/// why a room isn't available
#[derive(Debug, Clone)]
pub enum UnavailableReason {
    /// the room could not be found
    NotFound,

    /// the room is deleted
    Deleted,

    /// the room is quarantined
    Quarantined,

    /// some other mysterious failure reason
    // TODO: remove
    Other,
    // /// too many events were received and the room actor is backlogged
    // Backlogged,
}

impl UnavailableReason {
    /// whether `.ready()` should always fail when this reason is encountered
    ///
    /// otherwise, ready may continue waiting for the room to become available
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::NotFound | Self::Deleted | Self::Quarantined)
    }
}

// NOTE: maybe make members an option hashmap?
#[derive(Debug)]
pub struct RoomData {
    pub room: Box<Room>,

    /// all room members
    pub members: HashMap<UserId, CachedRoomMember>,

    /// all channels in this room
    pub channels: HashMap<ChannelId, CachedChannel>,

    /// all roles in this room
    pub roles: HashMap<RoleId, CachedRole>,

    /// loaded/active threads
    // TODO: use CachedChannel instead?
    pub threads: HashMap<ChannelId, CachedThread>,

    /// member lists in this room
    pub member_lists: HashMap<ListTarget, List>,

    /// whether room members have been loaded or not
    pub members_loaded: bool,
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
}

#[derive(Debug, Clone)]
pub struct CachedRoomMember {
    /// the room member
    pub member: RoomMember,

    /// the user associated with the room member
    pub user: Arc<User>,
}

// NOTE: maybe make members an option hashmap?
#[derive(Debug, Clone)]
pub struct CachedThread {
    /// the channel object for the thread
    pub thread: Channel,

    /// thread members
    pub members: HashMap<UserId, ThreadMember>,

    /// whether thread members have been loaded or not
    pub members_loaded: bool,
}

// TODO: use ChannelItem from crate-backend-services/src/channels.rs?
#[derive(Clone, Debug)]
pub struct CachedChannel {
    /// the channel itself
    pub inner: Channel,
    // /// channel permission overwrites as bitfields
    // pub overwrites: HashMap<Uuid, CachedPermissionOverwrite>,
}

#[derive(Clone, Debug)]
pub struct CachedRole {
    /// the role itself
    pub inner: Role,
    // /// allowed permissions as a bitfield
    // pub allow: PermissionBits,

    // /// denied permissions as a bitfield
    // pub deny: PermissionBits,
}

pub enum RoomEvent {
    Sync(MessageSync),
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
                RoomSnapshotState::Loaded(data) => !with_members || data.members_loaded,
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

    // /// create a subscription to a member list
    // pub fn member_list(&self, conn_id: ConnectionId) -> MemberList {
    //     todo!()
    // }
}

impl RoomWriter {
    pub fn handle(&self) -> RoomHandle {
        todo!()
    }

    // pub fn unload(self) { todo!() }
    // pub fn delete(self) { todo!() }
}

// TODO: move to common
/// get the room id for a message sync event
fn sync_room_id(sync: &MessageSync) -> Option<RoomId> {
    match sync {
        MessageSync::RoomCreate { room } => Some(room.id),
        MessageSync::ChannelCreate { channel } => channel.room_id,
        // TODO: handle more events
        _ => None,
    }
}

// TODO: move to common
fn sync_channel_id(sync: &MessageSync) -> Option<ChannelId> {
    match sync {
        MessageSync::ChannelCreate { channel } => Some(channel.id),
        // TODO: handle more events
        _ => None,
    }
}
