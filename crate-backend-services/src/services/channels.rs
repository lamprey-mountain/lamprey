use std::collections::HashMap;

use common::{
    v1::types::{
        Channel, PermissionOverwriteType, ThreadMember, preferences::PreferencesChannel, util::Time,
    },
    v2::types::{ChannelId, MessageId, UserId},
};
use lamprey_backend_core::types::permission::PermissionBits;
use uuid::Uuid;

use crate::prelude::*;

pub struct Service {
    globals: Globals,
    handles: HashMap<UserId, ChannelHandle>,
    private: HashMap<(ChannelId, UserId), Arc<ChannelPrivate>>,
    // typing: (),
    // idempotency_keys: (),
}

pub struct ChannelHandle {
    /// the channel itself
    pub inner: Box<Channel>,

    // /// channel permission overwrites as bitfields
    // pub overwrites: HashMap<Uuid, CachedPermissionOverwrite>,
    /// thread members
    ///
    /// is None if thread members haven't finished loading yet
    pub thread_members: Option<HashMap<UserId, ThreadMember>>,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub struct CachedPermissionOverwrite {
//     /// id of role or user
//     pub id: Uuid,

//     /// whether this is for a user or role
//     pub ty: PermissionOverwriteType,

//     /// allowed permissions as a bitfield
//     pub allow: PermissionBits,

//     /// denied permissions as a bitfield
//     pub deny: PermissionBits,
// }

impl ChannelHandle {
    // how would this work? i need to recursively get overwrites, including the room perms
    // pub fn permissions(&self) -> Permissions {
    //     todo!()
    // }

    // pub async fn edit(&self, ...) {}
    // pub async fn delete(&self, ...) {}
}

// struct MessagesHandle;

// impl MessagesHandle {
//     pub async fn send(&self, ...) {}
//     pub async fn get(&self, id: MessageId) {}
//     pub async fn delete(&self, id: MessageId) {}
// }

pub struct ChannelPrivate {
    /// the id of the last message this user has read
    pub read_marker_id: Option<MessageId>,

    /// when this user last read the pins
    pub pins_read_at: Option<Time>,

    /// the total number of mentions for this user in this channel
    pub mention_count: u64,

    /// the total number of unread messages for private channels
    pub unread_count: Option<u64>,

    // NOTE: this is also stored in preferences service
    pub preferences: Arc<PreferencesChannel>,

    pub slowmode_thread_expire_at: Option<Time>,
    pub slowmode_message_expire_at: Option<Time>,
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    // pub async fn create(&self, ...)
    // pub async fn edit(&self, ...)

    // pub async fn get(&self, _channel_id: ChannelId) -> ChannelHandle {
    pub async fn get(&self, _channel_id: ChannelId) {
        todo!()
    }

    // copy crate-backend/src/services/channel.rs
}
