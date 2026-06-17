use common::v1::types::{SessionToken, presence::Presence};
use url::Url;

use crate::prelude::*;
use crate::syncer::SyncerHandle;
use crate::{http::Http, syncer::Syncer};

/// for bots, contains single sync session
pub struct Client {
    syncer: Syncer,
    http: Http,
}

// /// configuration needed to start a new session
// pub struct ClientConfig {
//     token: SessionToken,
//     api_url: Url,
//     sync_url: Url,
//     cdn_url: Url,
// }

pub struct ClientBuilder {
    // TODO
}

impl ClientBuilder {
    /// set the token for this client
    pub fn token(token: SessionToken) -> Self {
        todo!()
    }

    /// set the api url for this client
    pub fn api_url(self, url: Url) -> Self {
        todo!()
    }

    /// set the sync url for this client
    pub fn sync_url(self, url: Url) -> Self {
        todo!()
    }

    /// set the cdn url for this client
    pub fn cdn_url(self, url: Url) -> Self {
        todo!()
    }

    /// set whether to autoconfigure `sync_url` and `cdn_url` from `api_url`
    pub fn autoconfig(self, enabled: bool) -> Self {
        todo!()
    }

    /// set the presence to send for sync
    pub fn presence(self, presence: Presence) -> Self {
        todo!()
    }

    /// build the client and connect to the syncer
    pub async fn build(self) -> Result<Client> {
        let http = Http::builder()
            // .token(token)
            // .api_url(url)
            .build()?;

        // if sync_url or cdn_url aren't set {
        //     if autoconfig is enabled {
        //         let info = http.server_info().await?;
        //         info.sync_url;
        //         info.cdn_url;
        //         populate sync url and cdn url
        //     } else {
        //         return error
        //     }
        // }

        let syncer = Syncer::builder()
            // .token(token)
            // .sync_url(url)
            // .presence(presence)
            .connect()
            .await?;

        todo!()
    }
}

impl Client {
    pub fn builder() -> ClientBuilder {
        todo!()
    }

    /// get a handle to the http client
    pub fn http(&self) -> Http {
        todo!()
    }

    /// get a handle to the syncer
    pub fn syncer(&self) -> SyncerHandle {
        todo!()
    }
}
