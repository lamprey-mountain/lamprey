use std::sync::Arc;

use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use utoipa::ToSchema;

use crate::{config::ConfigInternal, error::Result, ServerStateInner};

pub struct ServiceAdmin {
    state: Arc<ServerStateInner>,
    cache: RwLock<Option<ConfigInternal>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPurgeCache {
    pub targets: Vec<AdminPurgeCacheTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPurgeCacheResponse {
    pub stats: Vec<AdminPurgeCacheStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPurgeCacheStat {
    pub target: AdminPurgeCacheTarget,
    pub bytes_reclaimed: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum AdminPurgeCacheTarget {
    Channels,
    Embeds,
    Permissions,
    Rooms,
    Sessions,
    Users,
    // NOTE: add more targets here as caches are added!
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminCollectGarbage {
    pub targets: Vec<AdminCollectGarbageTarget>,
    pub mode: AdminCollectGarbageMode,

    /// whether to return 202 accepted or calculate stats
    #[serde(rename = "async")]
    pub async_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminCollectGarbageResponse {
    pub stats: Vec<AdminCollectGarbageStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminCollectGarbageStat {
    pub target: AdminCollectGarbageTarget,

    /// The number of milliseconds taken to run this garbage collection task.
    pub ms_elapsed: u64,

    /// The number of rows that were deleted (or would be deleted)
    pub rows_deleted: u64,

    /// Number of bytes deleted (or would be deleted); only returned for the `Media` target.
    // TODO: skip serializing if none
    pub bytes_deleted: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum AdminCollectGarbageTarget {
    Media,
    Messages,
    Session,
    AuditLog,
    RoomAnalytics,
    // NOTE: add more targets here as more garbage collectable resources are added!
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum AdminCollectGarbageMode {
    /// Dry-run mode. Calculate stats, but don't touch the database at all.
    Dry,

    /// Set `deleted_at` for all records that should be garbage collected
    Mark,

    /// Delete all records with `deleted_at` set. Note that `Mark` will need to be run first to do anything.
    Sweep,
}

impl ServiceAdmin {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache: RwLock::new(None),
        }
    }

    pub async fn get_config(&self) -> Result<ConfigInternal> {
        if let Some(config) = self.cache.read().await.as_ref() {
            return Ok(config.to_owned());
        }

        let config =
            self.state.data().config_get().await?.ok_or_else(|| {
                crate::Error::Internal("internal config not initialized".to_string())
            })?;

        *self.cache.write().await = Some(config.clone());
        Ok(config)
    }

    pub async fn verify_admin_token(&self, token: &str) -> bool {
        let Ok(config) = self.get_config().await else {
            return false;
        };

        let Some(admin_token) = config.admin_token else {
            return false;
        };

        if admin_token.len() != token.len() {
            return false;
        }

        admin_token.as_bytes().ct_eq(token.as_bytes()).into()
    }

    pub fn start_background_tasks(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let srv = state.services();
            if !state.config.enable_admin_token {
                let data = state.data();
                if let Ok(Some(mut config_internal)) = data.config_get().await {
                    config_internal.admin_token = None;
                    if let Ok(()) = data.config_put(config_internal.clone()).await {
                        *srv.admin.cache.write().await = Some(config_internal);
                    }
                }
                return;
            }

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let data = state.data();
                if let Ok(Some(mut config_internal)) = data.config_get().await {
                    let token = nanoid::nanoid!(32);
                    config_internal.admin_token = Some(token);
                    if let Err(err) = data.config_put(config_internal.clone()).await {
                        tracing::error!("failed to rotate admin token: {err:?}");
                    } else {
                        *srv.admin.cache.write().await = Some(config_internal);
                    }
                }
            }
        });
    }

    pub async fn collect_garbage(
        &self,
        req: AdminCollectGarbage,
    ) -> Result<AdminCollectGarbageResponse> {
        let mut stats = vec![];
        let data = self.state.data();

        for target in req.targets {
            let start_time = std::time::Instant::now();
            let (rows_deleted, bytes_deleted) = match target {
                AdminCollectGarbageTarget::Media => {
                    let (rows, bytes) = self.gc_media(req.mode).await?;
                    (rows, Some(bytes))
                }
                AdminCollectGarbageTarget::Messages => (data.gc_messages(req.mode).await?, None),
                AdminCollectGarbageTarget::Session => (data.gc_sessions(req.mode).await?, None),
                AdminCollectGarbageTarget::AuditLog => (data.gc_audit_logs(req.mode).await?, None),
                AdminCollectGarbageTarget::RoomAnalytics => {
                    (data.gc_room_analytics(req.mode).await?, None)
                }
            };

            stats.push(AdminCollectGarbageStat {
                target,
                ms_elapsed: start_time.elapsed().as_millis() as u64,
                rows_deleted,
                bytes_deleted,
            });
        }

        Ok(AdminCollectGarbageResponse { stats })
    }

    async fn gc_media(&self, mode: AdminCollectGarbageMode) -> Result<(u64, u64)> {
        let data = self.state.data();
        let blobs = &self.state.blobs;
        match mode {
            AdminCollectGarbageMode::Mark => {
                let rows = data.gc_media_mark().await?;
                Ok((rows, 0))
            }
            AdminCollectGarbageMode::Sweep => {
                let mut rows_deleted = 0;
                let mut bytes_deleted = 0;
                loop {
                    let media_to_delete = data.gc_media_get_sweep_candidates(50).await?;
                    if media_to_delete.is_empty() {
                        break;
                    }

                    for media_id in &media_to_delete {
                        let path = format!("media/{}/", media_id);
                        let items = blobs.list_with(&path).recursive(true).await?;
                        for item in items {
                            if item.metadata().is_file() {
                                let meta = blobs.stat(item.path()).await?;
                                bytes_deleted += meta.content_length();
                                blobs.delete(item.path()).await?;
                            }
                        }
                    }

                    let deleted = data.gc_media_delete_swept(&media_to_delete).await?;
                    rows_deleted += deleted;
                }
                Ok((rows_deleted, bytes_deleted))
            }
            AdminCollectGarbageMode::Dry => {
                // For mark: count what would be marked.
                // For sweep: count what would be swept, and their sizes.
                todo!()
            }
        }
    }

    pub async fn purge_caches(&self, req: AdminPurgeCache) -> Result<AdminPurgeCacheResponse> {
        let mut stats = vec![];
        let srv = self.state.services();

        for target in req.targets {
            // TODO: calculate bytes_reclaimed
            // moka does not seem to expose this?
            // maybe i should remove this; theres not an accurate way to calculate this considering Rc/Arc, allocator overhead, etc. And it wouldn't be *that* useful anyways.
            let bytes_reclaimed = 0;
            match target {
                AdminPurgeCacheTarget::Channels => srv.channels.purge_cache(),
                AdminPurgeCacheTarget::Embeds => srv.embed.purge_cache(),
                AdminPurgeCacheTarget::Permissions => srv.perms.purge_cache(),
                AdminPurgeCacheTarget::Rooms => srv.rooms.purge_cache(),
                AdminPurgeCacheTarget::Sessions => srv.sessions.purge_cache(),
                AdminPurgeCacheTarget::Users => srv.users.purge_cache(),
            }
            stats.push(AdminPurgeCacheStat {
                target,
                bytes_reclaimed,
            });
        }
        Ok(AdminPurgeCacheResponse { stats })
    }
}
