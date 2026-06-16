use std::sync::{Arc, Weak};

use lamprey_backend_core::config::Config;

mod rooms;

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

    // // TEMP: compat
    // database_compat: Box<PostgresPool>,

    // /// storage for large blobs
    // blobs: opendal::Operator,

    // /// send and receive messages
    // messaging: Messaging,
}

pub struct Services {
    pub rooms: rooms::Service,
}

impl Services {
    pub fn new(/* ... */) -> Self {
        todo!()
    }
}
