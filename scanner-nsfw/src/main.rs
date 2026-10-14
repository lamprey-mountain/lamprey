use axum::{
    Json,
    extract::State,
    routing::{get, post},
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;

use crate::{
    config::{Config, ListenConfig},
    error::ScanError,
};

mod cli;
mod config;
mod error;

use scanner_nsfw::Scanner;

struct AppState {
    scanner: Scanner,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct ScanRequest {
    path: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct ScanResponse {
    score: f32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct HealthResponse {
    ok: bool,
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "health",
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

#[utoipa::path(
    post,
    path = "/scan",
    request_body = ScanRequest,
    responses(
        (status = 200, description = "Scan completed", body = ScanResponse),
        (status = 400, description = "Scan failed"),
        (status = 404, description = "File not found"),
        (status = 500, description = "Internal error"),
    ),
    tag = "scan",
)]
async fn scan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, ScanError> {
    info!("scanning file: {}", req.path);
    let score = state
        .scanner
        .run(PathBuf::from(&req.path).as_path())
        .await?;
    Ok(Json(ScanResponse { score }))
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "NSFW Scanner API",
        description = "Scan images for NSFW content using an ONNX model",
    ),
    paths(health, scan),
    components(schemas(ScanRequest, ScanResponse, HealthResponse)),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "scan", description = "Image scanning endpoints"),
    ),
)]
struct ApiDoc;

async fn serve_transport(listen: ListenConfig, router: axum::Router) -> anyhow::Result<()> {
    match listen {
        ListenConfig::Tcp { address, port } => {
            let addr = SocketAddr::new(address, port);
            let listener = tokio::net::TcpListener::bind(addr).await?;
            info!("listening on http://{addr}");
            axum::serve(listener, router).await?;
        }
        ListenConfig::Unix { path } => {
            if let Some(p) = path.parent() {
                tokio::fs::create_dir_all(p).await?;
            }
            if path.exists() {
                tracing::warn!("deleting existing socket {}", path.display());
                tokio::fs::remove_file(&path).await?;
            }
            let listener = tokio::net::UnixListener::bind(&path)?;
            info!("listening on unix:{}", path.display());
            axum::serve(listener, router).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let config = Config::load(&cli.config)?;

    let subscriber = Registry::default()
        .with(EnvFilter::try_new(&config.rust_log)?)
        .with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(subscriber)?;

    info!("starting scanner-nsfw with RUST_LOG={}", config.rust_log);

    let model_path = config.models_path.join(&config.model);
    info!("loading model from {}", model_path.display());
    let scanner = Scanner::load(&model_path).await?;

    let state = Arc::new(AppState { scanner });

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .route("/health", get(health))
        .route("/scan", post(scan))
        .with_state(state)
        .split_for_parts();

    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::very_permissive());

    serve_transport(config.listen, router).await?;

    Ok(())
}
