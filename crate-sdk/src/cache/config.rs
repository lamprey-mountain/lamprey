use std::{sync::Arc, time::Duration};

use dashmap::DashMap;

use crate::cache::{Cache, CacheInner};

/// configuration for a [`Cache`]
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// the maximum number of users to store in the cache
    pub max_users: usize,

    /// the maximum number of non-room channels to store in the cache
    pub max_dms: usize,

    /// the maximum number of messages to store per channel
    pub max_messages: usize,

    /// how long to cache weakly held data for
    pub time_to_live: Duration,
}

#[derive(Debug, Default)]
pub struct CacheBuilder {
    config: CacheConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_users: 1024,
            max_dms: 1024,
            max_messages: 1024,
            time_to_live: Duration::from_secs(60 * 60),
        }
    }
}

impl CacheConfig {
    #[inline]
    pub fn new(self) -> Self {
        Self::default()
    }

    /// configure the maximum number of users to store in the cache
    #[inline]
    pub fn max_users(mut self, max_users: usize) -> Self {
        self.max_users = max_users;
        self
    }

    /// configure the maximum number of non-room channels to store in the cache
    #[inline]
    pub fn max_dms(mut self, max_dms: usize) -> Self {
        self.max_dms = max_dms;
        self
    }

    /// configure the maximum number of messages to store per channel
    #[inline]
    pub fn max_messages(mut self, max_messages: usize) -> Self {
        self.max_messages = max_messages;
        self
    }

    /// configure how long to cache weakly held data for
    #[inline]
    pub fn time_to_live(mut self, time_to_live: Duration) -> Self {
        self.time_to_live = time_to_live;
        self
    }
}

impl CacheBuilder {
    #[inline]
    pub fn new(self) -> Self {
        Self::default()
    }

    /// replace the cache config
    #[inline]
    pub fn config(mut self, config: CacheConfig) -> Self {
        self.config = config;
        self
    }

    /// configure the maximum number of users to store in the cache
    #[inline]
    pub fn max_users(mut self, max_users: usize) -> Self {
        self.config.max_users = max_users;
        self
    }

    /// configure the maximum number of non-room channels to store in the cache
    #[inline]
    pub fn max_dms(mut self, max_dms: usize) -> Self {
        self.config.max_dms = max_dms;
        self
    }

    /// configure the maximum number of messages to store per channel
    #[inline]
    pub fn max_messages(mut self, max_messages: usize) -> Self {
        self.config.max_messages = max_messages;
        self
    }

    /// configure how long to cache weakly held data for
    #[inline]
    pub fn time_to_live(mut self, time_to_live: Duration) -> Self {
        self.config.time_to_live = time_to_live;
        self
    }

    pub fn build(self) -> Cache {
        Cache {
            inner: Arc::new(CacheInner {
                config: self.config,
                rooms: DashMap::new(),
                channels: DashMap::new(),
                users: DashMap::new(),
            }),
        }
    }
}

impl From<CacheBuilder> for CacheConfig {
    fn from(value: CacheBuilder) -> Self {
        value.config
    }
}

impl From<&Cache> for CacheConfig {
    fn from(value: &Cache) -> Self {
        value.inner.config.clone()
    }
}

impl From<&CacheInner> for CacheConfig {
    fn from(value: &CacheInner) -> Self {
        value.config.clone()
    }
}

impl From<CacheInner> for CacheConfig {
    fn from(value: CacheInner) -> Self {
        value.config
    }
}
