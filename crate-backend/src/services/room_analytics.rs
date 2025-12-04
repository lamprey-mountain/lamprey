use std::sync::Arc;
use std::time::Duration;
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
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // every hour
            loop {
                interval.tick().await;
                info!("Taking room analytics snapshot");
                if let Err(e) = state.data().room_analytics_snapshot_all().await {
                    error!("Failed to take room analytics snapshot: {}", e);
                }
            }
        });
    }
}
