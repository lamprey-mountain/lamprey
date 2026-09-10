use crate::cache::Cache;
use crate::http::Http;
use crate::syncer::SyncerHandle;

mod builder;

pub use builder::ClientBuilder;

/// main entrypoint
pub struct Client {
    syncer: SyncerHandle,
    http: Http,

    #[cfg(feature = "cache")]
    cache: Option<Cache>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// get a handle to the http client
    pub fn http(&self) -> Http {
        self.http.clone()
    }

    /// get a handle to the syncer
    pub fn syncer(&self) -> SyncerHandle {
        self.syncer.handle()
    }

    /// get a handle to the cache
    // TODO: make this type safe viw Client<Cache> instead of returning Option
    // /// main entrypoint
    // pub struct Client<Cache> {
    //     syncer: SyncerHandle,
    //     http: Http,
    //     cache: Cache,
    // }
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Option<Cache> {
        self.cache.clone()
    }
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            // NOTE: handle() isn't quite the same as clone()
            // it resubscribes to the broadcast channel, which could cause messages to be missed.
            // this could lead to subtle race conditions!
            syncer: self.syncer.handle(),
            http: self.http.clone(),
            #[cfg(feature = "cache")]
            cache: self.cache.clone(),
        }
    }
}
