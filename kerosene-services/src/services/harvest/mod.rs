use std::{sync::Arc, time::Duration};

use crate::prelude::*;
use common::v1::types::{
    HarvestId, RoomId, UserId,
    error::{ApiError, ErrorCode},
    harvest::{Harvest, HarvestCreateRoom, HarvestCreateUser, HarvestStatus, HarvestType},
    util::Time,
};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

mod archive;
mod generate;

/// the maximum number of harvest jobs that can be running simultaneously
// TODO: make this configurable
const MAX_HARVEST_JOBS: usize = 2;

/// how long a completed harvest archive stays downloadable
pub(super) const EXPIRES_AFTER: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// how often to check for expired harvests
const GC_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub struct ServiceHarvest {
    globals: Globals,
}

impl ServiceHarvest {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }

    pub(super) fn start_background_tasks(&self) {
        let globals = self.globals.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(GC_INTERVAL).await;

                let expired = match globals.begin_read().await {
                    Ok(mut txn) => match txn.harvest_gc(Time::now_utc()).await {
                        Ok(expired) => expired,
                        Err(e) => {
                            error!("failed to fetch expired harvests: {e:?}");
                            continue;
                        }
                    },
                    Err(e) => {
                        error!("failed to acquire data for harvest gc: {e:?}");
                        continue;
                    }
                };

                for harvest in expired {
                    if let Err(e) = gc_harvest(&globals, harvest).await {
                        error!("failed to garbage collect harvest: {e:?}");
                    }
                }
            }
        });

        let globals = self.globals.clone();
        tokio::spawn(async move {
            let semaphore = Arc::new(Semaphore::new(MAX_HARVEST_JOBS));

            loop {
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => {
                        warn!("semaphore closed");
                        break;
                    }
                };

                let mut txn = match globals.begin().await {
                    Ok(db) => db,
                    Err(e) => {
                        error!("failed to acquire data for harvest loop: {:?}", e);
                        drop(permit);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                match txn.harvest_claim().await {
                    Ok(Some(harvest)) => {
                        let _ = txn.commit().await;
                        let globals2 = globals.clone();
                        tokio::spawn(async move {
                            if let Err(e) = globals2.services().harvest.generate(harvest).await {
                                error!("failed to generate harvest: {:?}", e);
                            }
                            drop(permit);
                        });
                    }
                    Ok(None) => {
                        let _ = txn.rollback().await;
                        drop(permit);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        error!("failed to claim harvest: {:?}", e);
                        drop(permit);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }

    /// queue a room harvest
    pub async fn create_room(
        &self,
        room_id: RoomId,
        harvest_options: &HarvestCreateRoom,
        requester_id: UserId,
    ) -> Result<Harvest> {
        let mut txn = self.globals.begin().await?;

        // keep old harvests around, but only one may be running at a time
        if let Some(existing) = txn.harvest_get_room(room_id).await? {
            if !existing.status.is_finished() {
                return Err(ApiError::from_code(ErrorCode::HarvestInProgress).into());
            }
        }

        let harvest = Harvest {
            id: HarvestId::new(),
            requester_id,
            queued_at: Time::now_utc(),
            status: HarvestStatus::Queued,
            ty: HarvestType::Room {
                target_room_id: room_id,
                create: harvest_options.clone(),
            },
        };
        txn.harvest_put(&harvest).await?;
        txn.commit().await?;

        Ok(harvest)
    }

    /// queue an user harvest
    pub async fn create_user(
        &self,
        user_id: UserId,
        harvest_options: &HarvestCreateUser,
        requester_id: UserId,
    ) -> Result<Harvest> {
        let mut txn = self.globals.begin().await?;

        // keep old harvests around, but only one may be running at a time
        if let Some(existing) = txn.harvest_get_user(user_id).await? {
            if !existing.status.is_finished() {
                return Err(ApiError::from_code(ErrorCode::HarvestInProgress).into());
            }
        }

        let harvest = Harvest {
            id: HarvestId::new(),
            requester_id,
            queued_at: Time::now_utc(),
            status: HarvestStatus::Queued,
            ty: HarvestType::User {
                target_user_id: user_id,
                create: harvest_options.clone(),
            },
        };
        txn.harvest_put(&harvest).await?;
        txn.commit().await?;

        Ok(harvest)
    }

    /// get a harvest
    pub async fn get(&self, harvest_id: HarvestId) -> Result<Harvest> {
        let mut db = self.globals.begin_read().await?;
        let harvest = db
            .harvest_get(harvest_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownHarvest))?;
        Ok(harvest)
    }

    /// get latest harvest for a user
    pub async fn get_user(&self, user_id: UserId) -> Result<Harvest> {
        let mut db = self.globals.begin_read().await?;
        let harvest = db
            .harvest_get_user(user_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownHarvest))?;
        Ok(harvest)
    }

    /// get latest harvest for a room
    pub async fn get_room(&self, room_id: RoomId) -> Result<Harvest> {
        let mut db = self.globals.begin_read().await?;
        let harvest = db
            .harvest_get_room(room_id)
            .await?
            .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownHarvest))?;
        Ok(harvest)
    }
}

/// delete an expired harvest's archive and record
async fn gc_harvest(globals: &Globals, harvest: Harvest) -> Result<()> {
    if let HarvestStatus::Completed { url, .. } = &harvest.status {
        match globals
            .blobs()
            .delete(url.path().trim_start_matches('/'))
            .await
        {
            Ok(()) => info!("deleted expired harvest archive {}", harvest.id),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => {}
            Err(e) => {
                warn!("failed to delete harvest archive {}: {e}", harvest.id);
                return Ok(());
            }
        }
    }

    // PERF: batch deletes
    let mut txn = globals.begin().await?;
    txn.harvest_delete(harvest.id).await?;
    txn.commit().await?;

    Ok(())
}
