// NOTE: should this (or part of this) be in -core or -common?

use std::sync::Weak;

use lamprey_backend_core::config::Config;
use lamprey_backend_data_postgres::data::{AnyData, Database};

use crate::{globals::messaging::Messaging, prelude::*, services::Services};

pub mod messaging;

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

    /// reference to the database for persistent data
    database: Box<dyn Database>,

    /// storage for large blobs
    blobs: opendal::Operator,

    /// send and receive messages
    messaging: Messaging,
}

impl GlobalsOwned {
    /// get a handle to the `Globals` itself
    pub fn handle(&self) -> Globals {
        Globals {
            inner: Arc::clone(&self.inner),
            services: Arc::downgrade(&self.services),
        }
    }
}

impl Globals {
    /// begin a database transaction
    ///
    /// use this for writes and for reads that need consistency
    pub async fn begin(&self) -> Result<AnyData> {
        self.inner.database.begin().await
    }

    /// begin a database session without a transaction
    ///
    /// use this for isolated single reads
    pub async fn begin_read(&self) -> Result<AnyData> {
        self.inner.database.begin_read().await
    }

    pub fn services(&self) -> Arc<Services> {
        self.services.upgrade().expect("Services should exist")
    }

    pub fn messaging(&self) -> &Messaging {
        &self.inner.messaging
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    // TEMP: i should write a wrapper for opendal
    pub fn blobs(&self) -> &opendal::Operator {
        &self.inner.blobs
    }
}
