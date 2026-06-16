// TODO: use this as the main entrypoint

use axum::Router;
use lamprey_backend_core::config::{Config, ListenComponent, ListenTransport};
use tokio::task::JoinSet;
use tracing::{info, warn};

use crate::{
    prelude::*,
    server::globals::{Globals, GlobalsOwned},
};

pub mod blobs;
pub mod globals;
pub mod messaging;

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

    /// get this server's state
    pub fn globals(&self) -> Globals {
        self.globals.handle()
    }

    /// start a server
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

        Ok(())
    }

    /// cleanly shutdown this server
    pub async fn shutdown(&mut self) -> Result<()> {
        self.listeners.shutdown().await;
        self.globals().services().shutdown().await;
        Ok(())
    }
}

// TODO: copy from crate-backend/src/serve/mod.rs

/// create an axum router for the api
pub fn create_router_api(_globals: Globals) -> Router {
    todo!()
}

/// create an axum router for metrics
pub fn create_router_metrics(_globals: Globals) -> Router {
    todo!()
}

/// create an axum router for the media server
pub fn create_router_media(_globals: Globals) -> Router {
    todo!()
}

/// create an axum router for redex http handlers
pub fn create_router_redexes(_globals: Globals) -> Router {
    // Router::new().layer(middleware::from_fn_with_state(globals, script_http))
    todo!()
}

/// serve an axum router on a transport
pub async fn serve_transport(transport: ListenTransport, router: Router) -> Result<()> {
    match transport {
        ListenTransport::Tcp { address, port } => {
            let listener = tokio::net::TcpListener::bind((address, port)).await?;
            axum::serve(listener, router).await?;
        }
        ListenTransport::Unix { path } => {
            if let Some(p) = path.parent() {
                tokio::fs::create_dir_all(p).await?;
            }
            if path.exists() {
                warn!("deleting existing socket {}", path.display());
                tokio::fs::remove_file(&path).await?;
            }
            let listener = tokio::net::UnixListener::bind(&path)?;
            let res = axum::serve(listener, router).await;
            let _ = tokio::fs::remove_file(path).await;
            res?;
        }
    }

    Ok(())
}
