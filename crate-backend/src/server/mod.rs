use axum::Router;
use lamprey_backend_core::config::{Config, ListenComponent};
use tokio::task::JoinSet;
use tracing::{info, warn};

use crate::{
    prelude::*,
    server::http::{
        apply_default_middleware, create_router_api, create_router_metrics, serve_transport,
    },
};

mod http;

#[cfg(feature = "webtransport")]
mod webtransport;

pub struct Server {
    globals: GlobalsOwned,
    listeners: JoinSet<Result<()>>,
    #[cfg(feature = "webtransport")]
    wt: webtransport::WtServer,
}

impl Server {
    /// setup a server
    pub async fn init_from_config(config: Config) -> Result<Self> {
        let globals = Globals::init_from_config(config).await?;
        Self::new(globals)
    }

    /// create a server from initialized `Globals`
    pub fn new(globals: GlobalsOwned) -> Result<Self> {
        #[cfg(feature = "webtransport")]
        let wt = webtransport::WtServer::new(globals.handle())?;

        Ok(Self {
            globals,
            listeners: JoinSet::new(),
            #[cfg(feature = "webtransport")]
            wt,
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
                    ListenComponent::Metrics => create_router_metrics(self.globals()),
                };
                router = router.merge(component_router);
            }
            router = apply_default_middleware(self.globals(), router);
            self.listeners
                .spawn(async move { serve_transport(transport, router).await });
            for c in &l.components {
                info!("{} listening on {}", c, l.transport);
            }
        }

        if self.listeners.is_empty() {
            warn!("no components enabled for any listeners");
        }

        #[cfg(feature = "webtransport")]
        {
            // TODO(?): refactor this to not clone WtServer
            // maybe make serve() for both Servers spawn a background task?
            let wt = self.wt.clone();
            tokio::spawn(async move {
                if let Err(e) = wt.serve().await {
                    warn!("webtransport server error: {e}");
                }
            });
        }

        while let Some(res) = self.listeners.join_next().await {
            res.unwrap()?;
        }

        Ok(())
    }

    /// cleanly shutdown this server
    pub async fn shutdown(&mut self) -> Result<()> {
        self.listeners.shutdown().await;
        #[cfg(feature = "webtransport")]
        self.wt.shutdown().await?;
        self.globals().services().shutdown().await;
        Ok(())
    }
}
