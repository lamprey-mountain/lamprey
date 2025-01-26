use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use handler::ErasedHandler;
use reqwest::Url;
use syncer::Syncer;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{error, warn};
use types::{
    Message, MessageClient, MessageCreateRequest, MessageEnvelope, MessagePayload, MessageSync, SessionToken, SyncResume, ThreadId
};

mod handler;
mod syncer;
mod http;

pub use handler::EventHandler;
pub use http::Http;

pub struct Client {
    pub syncer: Syncer,
    pub http: Http,
}

impl Client {
    pub fn new(token: SessionToken) -> Self {
        Self { http: Http::new(token.clone()), syncer: Syncer::new(token) }
    }

    // TODO: custom base url
    // pub fn with_base_url(self, base_url: Url) -> Self {
    //     Self {
    //         base_url,
    //         syncer: syncer.wi
    //         ..self
    //     }
    // }

    pub fn with_handler(self, handler: Box<dyn ErasedHandler>) -> Self {
        Self { syncer: self.syncer.with_handler(handler), ..self }
    }
}
