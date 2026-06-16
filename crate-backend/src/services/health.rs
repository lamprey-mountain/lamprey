use lamprey_backend_core::types::health::{Healthcheck, HealthcheckServices, HealthcheckStatus};

use crate::prelude::*;

pub struct ServiceHealth {
    state: ServerState2Handle,
}

impl ServiceHealth {
    pub fn new(state: ServerState2Handle) -> Self {
        Self { state }
    }

    /// generate a healthcheck for this server
    pub async fn healthcheck(&self) -> Healthcheck {
        let s = &self.state;

        let mut issues = vec![];
        issues.extend(s.config().lint());
        // TODO: issues for slow connections

        let health_database = todo!("check connection to database");
        // TODO: warn based on pool total/idle connections?
        // pool.size();
        // pool.num_idle();

        let health_blobs = s
            .blobs()
            .check()
            .await
            .map_or(HealthcheckStatus::Healthy, |_| HealthcheckStatus::Failed);
        // let health_messaging = s.messaging().is_connected();
        // let health_queue = s.messaging().is_connected();
        let health_messaging = todo!();
        let health_queue = todo!();
        let health_email = s
            .services()
            .email
            .test()
            .await
            .map_or(HealthcheckStatus::Healthy, |_| HealthcheckStatus::Failed);

        let services = HealthcheckServices {
            database: todo!(),
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
