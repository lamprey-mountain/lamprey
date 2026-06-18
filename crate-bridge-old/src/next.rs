use std::sync::{Arc, Weak};

use futures::Stream;
use tokio::sync::{broadcast, mpsc};

use crate::next::config::{Config, ConfigService};

pub struct GlobalsOwned {
    inner: Arc<GlobalsInner>,
    services: Arc<Services>,
}

pub struct Globals {
    inner: Arc<GlobalsInner>,
    services: Weak<Services>,
}

struct GlobalsInner {
    /// config for this server
    config: Box<Config>,

    /// reference to the database for persistent data
    database: Box<dyn Database>,
    // portal handle, ...
}

impl Globals {
    pub async fn init_from_config(config: Config) -> Result<GlobalsOwned> {
        let inner = Arc::new(GlobalsInner {
            config: todo!(),
            database: todo!(),
        });

        // create services and tie up the arc cycle
        let srv = Arc::new_cyclic(|weak_services| {
            let globals = Globals {
                inner: Arc::clone(&inner),
                services: weak_services.clone(),
            };
            Services::new(globals)
        });

        Ok(GlobalsOwned {
            inner,
            services: srv,
        })
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }
}
