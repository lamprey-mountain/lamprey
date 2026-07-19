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

#[cfg(any())]
mod old_next {
    use std::{collections::HashMap, sync::Arc};

    use common::v1::types::{
        ChannelId, MessageId, MessageVerId, UserId, preferences::PreferencesChannel, util::Time,
    };
    use moka::future::Cache;

    use crate::ServerStateInner;

    pub struct ServiceChannels {
        state: Arc<ServerStateInner>,

        /// private user data
        cache_private: Cache<(ChannelId, UserId), ChannelPrivate>,

        /// dm and gdm channels
        // other channel exist in room actors, but dms don't exist in rooms
        // merge recipients here
        cache_dms: Cache<ChannelId, Channel>,

        /// typing indicators
        typing: Cache<(ChannelId, UserId), TypingUser>,

        /// deduplicating channel create requests
        idempotency_keys: Cache<String, ChannelId>,

        slowmode_expire: Cache<(ChannelId, UserId, SlowmodeKind), Time>,
    }

    /// channel data for a user
    #[derive(Debug)]
    pub struct ChannelPrivate {
        pub unread_state: Option<AckState>,
        pub preferences: Option<PreferencesChannel>,
        pub slowmode_thread_expire_at: Option<Time>,
        pub slowmode_message_expire_at: Option<Time>,
        // does thread_member go here?
    }

    #[derive(Debug)]
    pub struct AckState {
        /// if this channel is unread
        // NOTE: this is separate because i might want to filter ignored/blocked users server side
        pub unread: bool,

        /// the id of the last message this user has read
        pub read_marker_id: Option<MessageId>,

        /// the version id of the last message this user has read
        pub read_marker_version: Option<MessageVerId>,

        /// the total number of mentions for this user in this channel
        pub mention_count: Option<u64>,

        /// the total number of unread messages for private channels
        pub unread_count: Option<u64>,

        /// when this user read the pins
        pub pins_read_at: Option<Time>,
        // how do unreads work with {calendar,wiki,document}?
        // calendar events could have their own ack state
    }

    // add these fields
    pub struct Channel {
        /// the id of the last message
        // TODO: use this instead of last_version_id in ui?
        pub last_message_id: Option<MessageId>,

        /// when a message was last pinned to this channel
        pub last_pin_timestamp: Option<Time>,
        // remove is_unread, last_read_id, mention_count, preferences, slowmode_thread_expire_at, slowmode_message_expire_at
    }

    pub struct TypingUser {
        pub expires_at: Time,
        // maybe later i can have "typing notification kinds", eg. "recording voice message"
    }

    pub enum SlowmodeKind {
        Message,
        Thread,
    }
}
