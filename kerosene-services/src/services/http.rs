use std::time::Duration;

// use common::util::routes::{Endpoint, Request};
use reqwest::{Client, Response};
use url::Url;

use crate::{
    error::{Error, Result},
    prelude::*,
};

pub struct ServiceHttp {
    // TEMP: make client public
    pub(crate) client: Client,
    state: Globals,
}

impl ServiceHttp {
    pub fn new(state: Globals) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent(
                state
                    .config()
                    .user_agent_header_value()
                    .expect("should always be valid user agent"),
            )
            .https_only(true)
            .build()
            .expect("failed to build http client");
        Self { client, state }
    }

    /// make a http GET request to this url
    pub async fn get(&self, url: Url) -> Result<Response> {
        let res = self.client.get(url).send().await?;

        if let Some(addr) = res.remote_addr() {
            for denied in &self.state.config().http.deny {
                if denied.contains(&addr.ip()) {
                    return Err(Error::BadStatic("url blacklisted"));
                }
            }
        } else {
            tracing::warn!("Could not get remote address for request.");
        }

        Ok(res.error_for_status()?)
    }

    // TODO: add more queries
    // oauth: get profile url with bearer token
    // oauth: post revocation url with bearer token
    // oauth: post exchange code for coken token url
    // media: post scan request to url
    // federation: send sync/ping
    // federation: get resource

    // TODO: fix and add this
    // /// send an http request
    // pub async fn send<E: Endpoint>(
    //     &self,
    //     req: E::Request,
    // ) -> ::core::result::Result<E::Response, Error> {
    //     let res = self.client.execute(req.encode()).await?;
    //     Ok(E::Response::extract(res).unwrap())
    // }
}
