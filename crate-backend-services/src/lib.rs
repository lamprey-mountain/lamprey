use std::sync::Weak;

use lamprey_backend_core::config::Config;

use crate::prelude::*;
use crate::services::Services;

pub mod services;

pub(crate) mod prelude {
    pub use crate::Globals;
    pub use lamprey_backend_core::prelude::*;
    pub use std::sync::Arc;
}

/// owned handle for the server's global state
#[derive(Clone)]
pub struct GlobalsOwned {
    inner: Arc<GlobalsInner>,
    services: Arc<Services>,
}

/// global state for the server
#[derive(Clone)]
pub struct Globals {
    inner: Arc<GlobalsInner>,
    services: Weak<Services>,
}

struct GlobalsInner {
    /// config for this server
    config: Box<Config>,
    // /// reference to the database for persistent data
    // database: Box<dyn Database>,

    // /// storage for large blobs
    // blobs: opendal::Operator,

    // /// send and receive messages
    // messaging: Messaging,
}
