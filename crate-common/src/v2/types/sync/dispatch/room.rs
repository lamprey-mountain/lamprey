use lamprey_macros::record;

use crate::v1::types::{
    Channel, Role, RoleId, RoleReorderItem, Room, RoomId, RoomMember, mirror::RoomSeq,
};

/// something happened in a room
///
/// requires a subscription to the room to receive
#[record]
#[serde(tag = "type")]
pub enum DispatchRoom {
    RoleCreate(RoleCreate),
    RoleUpdate(RoleUpdate),
    RoleDelete(RoleDelete),
    RoleReorder(RoleReorder),
}

/// a room was created and/or you joined a room
#[record]
pub struct RoomCreate {
    pub room: Box<Room>,
    pub roles: Vec<Role>,
    pub channels: Vec<Channel>,
    pub threads: Vec<Channel>,

    /// your own room member
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_member: Option<Box<RoomMember>>,

    /// the room sync sequence number of this event, for offline sync
    // TODO: this should probably be a part of room
    pub seq: RoomSeq,
}

/// a room was updated
#[record]
pub struct RoomUpdate {
    pub room: Box<Room>,
    pub seq: Option<RoomSeq>,
}

/// a room was deleted, you left a room, or you were removed (kicked/banned) from a room
#[record]
pub struct RoomDelete {
    pub room_id: RoomId,
    pub seq: RoomSeq,
}

/// a role was created
#[record]
pub struct RoleCreate {
    pub role: Box<Role>,
    pub seq: RoomSeq,
}

/// a role was updated
#[record]
pub struct RoleUpdate {
    pub role: Box<Role>,
    pub seq: RoomSeq,
}

/// a role was deleted
#[record]
pub struct RoleDelete {
    pub room_id: RoomId,
    pub role_id: RoleId,
    pub seq: RoomSeq,
}

/// the role hierarchy was reordered
#[record]
pub struct RoleReorder {
    pub room_id: RoomId,
    pub roles: Vec<RoleReorderItem>,
    pub seq: RoomSeq,
}

impl DispatchRoom {
    /// id of the room this event happened in
    pub fn room_id(&self) -> RoomId {
        match self {
            DispatchRoom::RoleCreate(a) => a.role.room_id,
            DispatchRoom::RoleUpdate(a) => a.role.room_id,
            DispatchRoom::RoleDelete(a) => a.room_id,
            DispatchRoom::RoleReorder(a) => a.room_id,
        }
    }

    /// the room sync sequence number of this event
    ///
    /// if None, this event doesnt increment the seq. used for offline sync.
    pub fn seq(&self) -> Option<RoomSeq> {
        match self {
            DispatchRoom::RoleCreate(a) => Some(a.seq),
            DispatchRoom::RoleUpdate(a) => Some(a.seq),
            DispatchRoom::RoleDelete(a) => Some(a.seq),
            DispatchRoom::RoleReorder(a) => Some(a.seq),
        }
    }
}
