use std::time::Duration;

use common::{util::routes::Endpoint, v1::types::federation::Hostname};
use reqwest::{Client, Response};
use url::Url;

use crate::prelude::*;

pub struct Service {
    client: Client,
}

impl Service {
    pub fn new(globals: Globals) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent(
                globals
                    .config()
                    .user_agent_header_value()
                    .expect("should always be valid user agent"),
            )
            .https_only(true)
            .build()
            .expect("failed to build http client");
        Self { client }
    }

    /// send an unauthenticated http GET request to this url
    pub async fn get(&self, _url: Url) -> Result<Response> {
        todo!()
    }

    /// get the reqwest client
    pub fn client(&self) -> reqwest::Client {
        todo!()
    }

    /// build a federated http request
    pub fn federated(&self) -> FederatedRequestBuilder {
        todo!()
    }
}

pub struct FederatedRequestBuilder {
    globals: Globals,
    // hostname is required
    hostname: Option<Hostname>,
}

impl FederatedRequestBuilder {
    /// set the destination hostname
    pub fn hostname(self, _hostname: Hostname) -> Self {
        todo!()
    }

    /// send an api request
    pub async fn send<E: Endpoint>(self, _req: E::Request) -> Result<E::Response> {
        // 1. lookup hostname urls
        // 2. get local keys
        // 3. construct OutgoingRequest, sign with key
        // 4. send request
        // 5. parse response
        todo!()
    }

    /// send a request to the `cdn_url`
    pub async fn send_cdn<E: Endpoint>(self, _req: E::Request) -> Result<E::Response> {
        todo!()
    }
}
