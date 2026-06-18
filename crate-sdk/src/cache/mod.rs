use std::ops::RangeBounds;

use async_trait::async_trait;

pub trait Cache: Send + Sync {
    // TODO: design this trait
}

#[async_trait]
pub trait CacheMap<K, V> {
    type Error;

    /// lookup a key
    async fn get(&self, k: K) -> Result<Option<V>, Self::Error>;

    /// get all keys and values in a range
    // PERF: maybe make this return a stream
    async fn all(&self, range: impl RangeBounds<K>) -> Result<Vec<(K, V)>, Self::Error>;

    /// insert or replace an entry
    async fn insert(&self, k: K, v: K) -> Result<(), Self::Error>;

    /// delete an entry
    async fn delete(&self, k: K) -> Result<(), Self::Error>;

    /// delete all entries
    async fn clear() -> Result<(), Self::Error>;
}

pub mod memory;
// pub mod idb; // TODO: webassembly indexeddb (feature = "wasm")
// pub mod sqlite; // TODO: webassembly indexeddb (feature = "sqlite")
