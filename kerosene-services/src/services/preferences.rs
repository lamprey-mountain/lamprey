use common::{
    v1::types::preferences::{
        PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser,
    },
    v2::types::{ChannelId, RoomId, UserId},
};

use crate::prelude::*;

pub struct Service {
    // preferences_global: Cache<UserId, Arc<PreferencesGlobal>>,
    // preferences_room: Cache<(RoomId, UserId), Arc<PreferencesRoom>>,
    // preferences_channel: Cache<(ChannelId, UserId), Arc<PreferencesChannel>>,
    // preferences_user: Cache<(UserId, UserId), Arc<PreferencesUser>>,
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    /// get a user's global config from the cache, loading from the database if not present
    pub async fn get_global(&self, _user_id: UserId) -> Result<Arc<PreferencesGlobal>> {
        todo!()
    }

    /// get a user's room config from the cache, loading from the database if not present
    pub async fn get_room(
        &self,
        _user_id: UserId,
        _room_id: RoomId,
    ) -> Result<Arc<PreferencesRoom>> {
        todo!()
    }

    /// get a user's channel config from the cache, loading from the database if not present
    pub async fn get_channel(
        &self,
        _user_id: UserId,
        _channel_id: ChannelId,
    ) -> Result<Arc<PreferencesChannel>> {
        todo!()
    }

    /// get a user's config for another user from the cache, loading from the database if not present
    pub async fn get_user(
        &self,
        _user_id: UserId,
        _other_id: UserId,
    ) -> Result<Arc<PreferencesUser>> {
        todo!()
    }
}
