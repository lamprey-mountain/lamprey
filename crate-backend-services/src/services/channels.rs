use std::collections::HashMap;

use common::{
    v1::types::{Channel, ThreadMember},
    v2::types::{ChannelId, UserId},
};

use crate::prelude::*;

pub struct Service {
    //
}

pub struct ChannelItem {
    /// the channel itself
    pub inner: Box<Channel>,
    // /// channel permission overwrites as bitfields
    // pub overwrites: HashMap<Uuid, CachedPermissionOverwrite>,
    /// thread members
    pub thread_members: Option<HashMap<UserId, ThreadMember>>,
}

impl ChannelItem {
    // pub fn permissions(&self) -> Permissions {}
    //
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    // pub async fn create(&self, ...)
    // pub async fn edit(&self, ...)

    pub async fn get(&self, _channel_id: ChannelId) {
        todo!()
    }

    // copy crate-backend/src/services/channel.rs
}
