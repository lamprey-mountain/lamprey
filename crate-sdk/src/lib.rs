use common::v1::types::SessionToken;
use handler::ErasedHandler;
use syncer::Syncer;

mod handler;
mod http;
mod syncer;

pub use handler::EventHandler;
pub use http::Http;

pub struct Client {
    pub syncer: Syncer,
    pub http: Http,
}

impl Client {
    pub fn new(token: SessionToken) -> Self {
        Self {
            http: Http::new(token.clone()),
            syncer: Syncer::new(token),
        }
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
        Self {
            syncer: self.syncer.with_handler(handler),
            ..self
        }
    }
}
