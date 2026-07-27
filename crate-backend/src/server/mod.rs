use axum::Router;
use lamprey_backend_core::config::{Config, ListenComponent};
use tokio::task::JoinSet;
use tracing::{info, warn};

use crate::{
    prelude::*,
    server::http::{create_router_api, serve_transport},
};

mod http;

pub struct Server {
    globals: GlobalsOwned,
    listeners: JoinSet<Result<()>>,
}

impl Server {
    /// setup a server
    pub async fn init_from_config(config: Config) -> Result<Self> {
        let globals = Globals::init_from_config(config).await?;
        Self::new(globals)
    }

    /// create a server from initialized `Globals`
    pub fn new(globals: GlobalsOwned) -> Result<Self> {
        Ok(Self {
            globals,
            listeners: JoinSet::new(),
        })
    }

    /// get a handle to the server's global state
    pub fn globals(&self) -> Globals {
        self.globals.handle()
    }

    /// start the server
    pub async fn serve(&mut self) -> Result<()> {
        info!("starting server");

        let globals = self.globals();
        for l in &globals.config().listen {
            let mut router = Router::new();
            let transport = l.transport.clone();
            for c in &l.components {
                let component_router = match c {
                    ListenComponent::Api => create_router_api(self.globals()),
                    ListenComponent::Metrics => todo!(),
                };
                router = router.merge(component_router);
            }
            self.listeners
                .spawn(async move { serve_transport(transport, router).await });
            for c in &l.components {
                info!("{} listening on {}", c, l.transport);
            }
        }

        if self.listeners.is_empty() {
            warn!("no components enabled for any listeners");
        }

        while let Some(res) = self.listeners.join_next().await {
            res.unwrap()?;
        }

        Ok(())
    }

    /// cleanly shutdown this server
    pub async fn shutdown(&mut self) -> Result<()> {
        self.listeners.shutdown().await;
        self.globals().services().shutdown().await;
        Ok(())
    }
}
