use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use http::StatusCode;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct Ratelimiter {
    buckets: Arc<RwLock<HashMap<RatelimitBucket, Ratelimit>>>,
}

#[derive(Clone, Debug)]
pub struct Ratelimit {
    retry_at: Option<SystemTime>,
    retry_after: Option<Duration>,
    limit: u64,
    remaining: u64,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RatelimitBucket(String);

impl Ratelimiter {
    // "i'm sending another request, get ratelimit info"
    pub async fn claim(&self, bucket: RatelimitBucket) {
        let _map = self.buckets.write().await;
        // map.entry(bucket);
        todo!()
    }

    // "i was ratelimited, what should i do next"
    pub fn handle_ratelimit(&self, req: &http::request::Parts, res: &http::response::Parts) {
        // if parts.status == StatusCode::TOO_MANY_REQUESTS {}
        todo!()
    }

    // update ratelimit state
    pub fn update(&self, res: &http::response::Parts) {
        todo!()
    }
}

impl RatelimitBucket {
    // TODO
}
