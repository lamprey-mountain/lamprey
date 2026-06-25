use tracing::info;

use crate::prelude::*;

/// the api server
pub struct Server {
    globals: GlobalsOwned,
    listeners: JoinSet<Result<()>>,
}

impl Server {
    /// initialize a server from some config
    pub async fn init_from_config(config: Config) -> Result<Self> {
        todo!()
    }

    /// create a server from initialized `GlobalsOwned`
    pub fn new(globals: GlobalsOwned) -> Result<Self> {
        Ok(Self {
            globals,
            listeners: JoinSet::new(),
        })
    }

    /// get this server's state
    pub fn globals(&self) -> Globals {
        self.globals.handle()
    }

    pub async fn serve(&self) -> Result<()> {
        info!("starting server");
        todo!()
    }

    /// cleanly shutdown this server
    pub async fn shutdown(&mut self) -> Result<()> {
        todo!()
    }
}
