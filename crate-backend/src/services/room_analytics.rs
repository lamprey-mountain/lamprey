use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{error, info};

use crate::error::Result;
use crate::ServerStateInner;

pub struct ServiceRoomAnalytics {
    state: Arc<ServerStateInner>,
}

impl ServiceRoomAnalytics {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub async fn snapshot_all(&self) -> Result<()> {
        info!("Taking room analytics snapshot");
        self.state.data().room_analytics_snapshot_all().await
    }

    pub fn spawn_snapshot_task(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            if let Ok(Some(last)) = state.data().room_analytics_get_last_snapshot_ts().await {
                let last_utc = last.assume_utc();
                let now = OffsetDateTime::now_utc();
                let elapsed = now - last_utc;
                let hour = time::Duration::hours(1);

                if elapsed < hour {
                    let wait_time = hour - elapsed;
                    if wait_time.is_positive() {
                        if let Ok(duration) = wait_time.try_into() {
                            tokio::time::sleep(duration).await;
                        }
                    }
                }
            }

            loop {
                info!("Taking room analytics snapshot");
                if let Err(e) = state.data().room_analytics_snapshot_all().await {
                    error!("Failed to take room analytics snapshot: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    }
}
