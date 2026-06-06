use std::sync::Arc;

use crate::{error::Result, ServerStateInner};
use common::v1::types::{
    error::{ApiError, ErrorCode},
    harvest::{Harvest, HarvestCreateRoom, HarvestCreateUser},
    HarvestId, RoomId, UserId,
};

/// the maximum number of harvest jobs that can be running simultaneously
const _MAX_HARVEST_JOBS: usize = 2;

pub struct ServiceHarvest {
    state: Arc<ServerStateInner>,
}

impl ServiceHarvest {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub(super) fn start_background_tasks(&self) {
        tokio::spawn(async {
            // loop { }
            // 1. poll harvest table for pending harvests
            // 2. call generate
        });
    }

    /// queue a room harvest
    pub async fn create_room(
        &self,
        _room_id: RoomId,
        _harvest: &HarvestCreateRoom,
    ) -> Result<Harvest> {
        // 1. get existing harvest if any
        // 2. update or create harvest, insert into db
        todo!()
    }

    /// queue an user harvest
    pub async fn create_user(
        &self,
        _user_id: UserId,
        _harvest: &HarvestCreateUser,
    ) -> Result<Harvest> {
        // 1. get existing harvest if any
        // 2. update or create harvest, insert into db
        todo!()
    }

    /// generate a harvest
    async fn _generate(&self, _harvest: Harvest) -> Result<()> {
        // 1. claim harvest (use postgres as queue)
        // 2. create temp sqlite file
        // 3. open with rusqlite
        // 4. fully paginate through all data, write to sqlite
        // 5. flush, close file
        // 6. upload sqlite db with media service
        // 7. update harvest (update postgres)
        // 8. emit message sync event
        todo!()
    }

    /// get a harvest
    pub async fn get(&self, harvest_id: HarvestId) -> Result<Harvest> {
        let mut db = self.state.acquire_data().await?;
        let harvest = db
            .harvest_get(harvest_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownHarvest))?;
        Ok(harvest)
    }

    /// get latest harvest for a user
    pub async fn get_user(&self, user_id: UserId) -> Result<Harvest> {
        let mut db = self.state.acquire_data().await?;
        let harvest = db
            .harvest_get_user(user_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownHarvest))?;
        Ok(harvest)
    }

    /// get latest harvest for a room
    pub async fn get_room(&self, room_id: RoomId) -> Result<Harvest> {
        let mut db = self.state.acquire_data().await?;
        let harvest = db
            .harvest_get_room(room_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownHarvest))?;
        Ok(harvest)
    }
}
