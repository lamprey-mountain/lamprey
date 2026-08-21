//! http client for the rest api

use std::sync::Arc;

use crate::prelude::*;
use common::{
    util::routes::{Endpoint, Request, Response},
    v1::types::SessionToken,
    v2::types::UserId,
};
use headers::HeaderMapExt;
use reqwest::header::HeaderMap;
use url::Url;

mod ratelimiter;
mod routes;

pub use routes::MessageCreateOptions;

/// an http client for interacting with the rest api
#[derive(Debug, Clone)]
pub struct Http {
    config: Arc<HttpConfig>,
    client: reqwest::Client,
}

#[derive(Debug)]
struct HttpConfig {
    token: SessionToken,
    api_url: Url,
    cdn_url: Url,
}

#[derive(Debug, Default)]
pub struct HttpBuilder {
    token: Option<SessionToken>,
    api_url: Option<Url>,
    cdn_url: Option<Url>,
}

impl HttpBuilder {
    pub fn token(mut self, token: SessionToken) -> Self {
        self.token = Some(token);
        self
    }

    pub fn api_url(mut self, url: Url) -> Self {
        self.api_url = Some(url);
        self
    }

    pub fn cdn_url(mut self, url: Url) -> Self {
        self.cdn_url = Some(url);
        self
    }

    pub fn build(self) -> Result<Http> {
        let token = self
            .token
            .ok_or_else(|| Error::MissingBuilderField("token".to_string()))?;
        let api_url = self
            .api_url
            .ok_or_else(|| Error::MissingBuilderField("api_url".to_string()))?;
        let cdn_url = self
            .cdn_url
            .ok_or_else(|| Error::MissingBuilderField("cdn_url".to_string()))?;

        let mut h = HeaderMap::new();
        h.typed_insert(headers::Authorization::bearer(&token.0).map_err(|_| Error::InvalidHeader)?);

        let client = reqwest::Client::builder().default_headers(h).build()?;

        Ok(Http {
            config: Arc::new(HttpConfig {
                token,
                api_url,
                cdn_url,
            }),
            client,
        })
    }
}

impl Http {
    pub fn builder() -> HttpBuilder {
        HttpBuilder::default()
    }

    pub fn api_url(&self) -> &Url {
        &self.config.api_url
    }

    pub fn cdn_url(&self) -> &Url {
        &self.config.cdn_url
    }

    pub fn token(&self) -> &SessionToken {
        &self.config.token
    }

    pub fn for_puppet(&self, id: UserId) -> Result<Self> {
        let mut h = HeaderMap::new();
        h.typed_insert(
            headers::Authorization::bearer(&self.token().0).map_err(|_| Error::InvalidHeader)?,
        );
        h.insert(
            "x-puppet-id",
            id.to_string()
                .try_into()
                .map_err(|_| Error::InvalidHeader)?,
        );
        let client = reqwest::Client::builder().default_headers(h).build()?;
        Ok(Self {
            client,
            ..self.clone()
        })
    }

    pub async fn dispatch<E: Endpoint>(&self, req: E::Request) -> Result<E::Response> {
        let http_req = req.encode();
        let res = self.client.execute(http_req.try_into()?).await?;

        let status = res.status();
        let headers = res.headers().clone();
        let body = res.bytes().await?;

        // PERF: stream body for some routes? (this is only relevant for media.)
        // TODO: better error handling

        let mut builder = http::Response::builder().status(status);
        *builder.headers_mut().expect("builder not errored") = headers;
        let http_res = builder.body(body).unwrap();

        Ok(E::Response::extract(http_res).unwrap())
    }
}
