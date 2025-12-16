use std::{sync::Arc, time::Duration};

use reqwest::{Client, Response};
use url::Url;

use crate::{
    error::{Error, Result},
    ServerStateInner,
};

pub struct ServiceHttp {
    client: Client,
    state: Arc<ServerStateInner>,
}

impl ServiceHttp {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::limited(10))
            // TODO: rename `url_preview.user_agent` -> `http.user_agent` since this will be used for all http requests
            .user_agent(&state.config.url_preview.user_agent)
            .https_only(true)
            .build()
            .expect("failed to build http client");
        Self { client, state }
    }

    /// make a http GET request to this url
    pub async fn get(&self, url: Url) -> Result<Response> {
        let res = self.client.get(url).send().await?;

        if let Some(addr) = res.remote_addr() {
            // TODO: rename `url_preview.deny` -> `http.deny` since this will be used for all http requests
            for denied in &self.state.config.url_preview.deny {
                if denied.contains(&addr.ip()) {
                    return Err(Error::BadStatic("url blacklisted"));
                }
            }
        } else {
            tracing::warn!("Could not get remote address for request.");
        }

        Ok(res.error_for_status()?)
    }
}
