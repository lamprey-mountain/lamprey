use common::v1::types::{SessionToken, presence::Presence};
use url::Url;

use crate::cache::{Cache, CacheSettings};
use crate::http::Http;
use crate::prelude::*;
use crate::syncer::{Syncer, SyncerHandle};

/// main entrypoint
pub struct Client {
    syncer: SyncerHandle,
    http: Http,
    #[cfg(feature = "cache")]
    cache: Option<Cache>,
}

pub struct ClientBuilder {
    token: Option<SessionToken>,
    api_url: Option<Url>,
    sync_url: Option<Url>,
    cdn_url: Option<Url>,
    autoconfig: bool,
    presence: Option<Presence>,
    #[cfg(feature = "cache")]
    cache: ClientBuilderCacheSettings,
}

#[cfg(feature = "cache")]
pub enum ClientBuilderCacheSettings {
    /// do not create a cache
    Disabled,

    /// create a cache with default settings
    Default,

    /// create a cache with these settings
    Configured(Box<CacheSettings>),
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            token: None,
            api_url: None,
            sync_url: None,
            cdn_url: None,
            autoconfig: true,
            presence: None,
            #[cfg(feature = "cache")]
            cache: ClientBuilderCacheSettings::Default,
        }
    }
}

impl ClientBuilder {
    /// set the token for this client
    pub fn token(mut self, token: SessionToken) -> Self {
        self.token = Some(token);
        self
    }

    /// set the api url for this client
    pub fn api_url(mut self, url: Url) -> Self {
        self.api_url = Some(url);
        self
    }

    /// set the sync url for this client
    pub fn sync_url(mut self, url: Url) -> Self {
        self.sync_url = Some(url);
        self
    }

    /// set the cdn url for this client
    pub fn cdn_url(mut self, url: Url) -> Self {
        self.cdn_url = Some(url);
        self
    }

    /// set whether to autoconfigure `sync_url` and `cdn_url` from `api_url`
    pub fn autoconfig(mut self, enabled: bool) -> Self {
        self.autoconfig = enabled;
        self
    }

    /// set the presence to send for sync
    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// disable the cache
    #[cfg(feature = "cache")]
    pub fn without_cache(mut self) -> Self {
        self.cache = ClientBuilderCacheSettings::Disabled;
        self
    }

    /// enable the cache with specific settings
    #[cfg(feature = "cache")]
    pub fn cache<S: Into<CacheSettings>>(mut self, settings: S) -> Self {
        self.cache = ClientBuilderCacheSettings::Configured(Box::new(settings.into()));
        self
    }

    /// build the client and connect to the syncer
    pub async fn build(self) -> Result<Client> {
        let token = self
            .token
            .ok_or_else(|| Error::Other("missing token".to_string()))?;
        let api_url = self
            .api_url
            .ok_or_else(|| Error::Other("missing api_url".to_string()))?;

        let http = Http::builder()
            .token(token.clone())
            .api_url(api_url.clone())
            .cdn_url(api_url.clone()) // cdn_url isn't used
            .build()?;

        let mut sync_url = self.sync_url;
        let mut cdn_url = self.cdn_url;

        if sync_url.is_none() || cdn_url.is_none() {
            if self.autoconfig {
                let info = http
                    .server_info()
                    .await
                    .map_err(|e| Error::Other(e.to_string()))?;
                if sync_url.is_none() {
                    sync_url = Some(info.sync_url);
                }
                if cdn_url.is_none() {
                    cdn_url = Some(info.cdn_url);
                }
            } else {
                return Err(Error::Other(
                    "missing sync_url or cdn_url and autoconfig is disabled".to_string(),
                ));
            }
        }

        let cdn_url = cdn_url.ok_or_else(|| Error::Other("missing cdn_url".to_string()))?;
        let http = Http::builder()
            .token(token.clone())
            .api_url(api_url.clone())
            .cdn_url(cdn_url.clone())
            .build()?;

        let sync_url = sync_url.ok_or_else(|| Error::Other("missing sync_url".to_string()))?;

        let syncer_builder = Syncer::builder().token(token).sync_url(sync_url);

        let syncer_builder = if let Some(presence) = self.presence {
            syncer_builder.presence(presence)
        } else {
            syncer_builder
        };

        let syncer = syncer_builder.build()?;

        #[cfg(feature = "cache")]
        let cache = match self.cache {
            ClientBuilderCacheSettings::Disabled => None,
            ClientBuilderCacheSettings::Default => Some(Cache::builder().build()),
            ClientBuilderCacheSettings::Configured(settings) => {
                Some(Cache::builder().settings(*settings).build())
            }
        };

        Ok(Client {
            syncer,
            http,
            #[cfg(feature = "cache")]
            cache,
        })
    }
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
