use lamprey_backend_core::types::health::{Healthcheck, HealthcheckServices, HealthcheckStatus};

use crate::prelude::*;

pub struct ServiceHealth {
    globals: Globals,
}

impl ServiceHealth {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }

    /// generate a healthcheck for this server
    pub async fn healthcheck(&self) -> Healthcheck {
        let s = &self.globals;

        let mut issues = vec![];
        issues.extend(s.config().lint());

        // TODO: issues for slow connections
        // TODO: warn based on db pool total/idle connections
        // pool.size();
        // pool.num_idle();

        let health_database = if s.temp_test_database().await {
            HealthcheckStatus::Healthy
        } else {
            HealthcheckStatus::Failed
        };

        let health_blobs = s
            .blobs()
            .check()
            .await
            .map_or(HealthcheckStatus::Healthy, |_| HealthcheckStatus::Failed);
        let health_messaging = if s.messaging().is_connected() {
            HealthcheckStatus::Healthy
        } else {
            HealthcheckStatus::Failed
        };

        let health_queue = if s.messaging().is_connected() {
            HealthcheckStatus::Healthy
        } else {
            HealthcheckStatus::Failed
        };

        let health_email = s
            .services()
            .email
            .test()
            .await
            .map_or(HealthcheckStatus::Healthy, |_| HealthcheckStatus::Failed);

        let services = HealthcheckServices {
            database: health_database,
            object_store: health_blobs,
            messaging: health_messaging,
            queue: health_queue,
            email: health_email,
        };
        let health = Healthcheck {
            status: services.overall(),
            services,
            issues,
        };
        health
    }
}
