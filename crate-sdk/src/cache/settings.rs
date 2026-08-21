use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::cache::{Cache, CacheInner};

/// configuration for a [`Cache`]
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// the maximum number of users to store in the cache
    pub max_users: usize,

    /// the maximum number of non-room channels to store in the cache
    pub max_dms: usize,

    /// the maximum number of messages to store per channels
    pub max_messages: usize,

    /// how long to cache weakly held data for
    pub time_to_live: Duration,
}

#[derive(Debug, Default)]
pub struct CacheBuilder {
    settings: CacheSettings,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            max_users: 1024,
            max_dms: 1024,
            max_messages: 1024,
            time_to_live: Duration::from_secs(60 * 60),
        }
    }
}

impl CacheBuilder {
    pub fn settings(mut self, settings: CacheSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn max_users(mut self, max_users: usize) -> Self {
        self.settings.max_users = max_users;
        self
    }

    pub fn max_dms(mut self, max_dms: usize) -> Self {
        self.settings.max_dms = max_dms;
        self
    }

    pub fn max_messages(mut self, max_messages: usize) -> Self {
        self.settings.max_messages = max_messages;
        self
    }

    pub fn time_to_live(mut self, time_to_live: Duration) -> Self {
        self.settings.time_to_live = time_to_live;
        self
    }

    pub fn build(self) -> Cache {
        Cache {
            inner: Arc::new(CacheInner {
                settings: self.settings,
                rooms: HashMap::new(),
                channels: HashMap::new(),
                users: HashMap::new(),
            }),
        }
    }
}

impl From<CacheBuilder> for CacheSettings {
    fn from(value: CacheBuilder) -> Self {
        value.settings
    }
}

impl From<&Cache> for CacheSettings {
    fn from(value: &Cache) -> Self {
        value.inner.settings.clone()
    }
}

impl From<&CacheInner> for CacheSettings {
    fn from(value: &CacheInner) -> Self {
        value.settings.clone()
    }
}

impl From<CacheInner> for CacheSettings {
    fn from(value: CacheInner) -> Self {
        value.settings
    }
}
