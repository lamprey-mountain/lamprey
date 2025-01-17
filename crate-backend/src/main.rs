use std::{sync::Arc, time::Duration};

use axum::{extract::DefaultBodyLimit, response::Html, routing::get, Json};
use dashmap::DashMap;
use data::{postgres::Postgres, Data};
use services::Services;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::broadcast::Sender;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;
use types::{MediaId, MediaUpload, MessageServer};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable as _};

pub mod data;
pub mod error;
mod routes;
pub mod types;
pub mod services;

use error::Result;

#[derive(OpenApi)]
#[openapi(components(schemas(
    types::Room,
    types::RoomPatch,
    types::User,
    types::Thread,
    types::ThreadPatch,
    types::Message,
    types::RoomMember,
    types::Role,
)))]
struct ApiDoc;

#[derive(Clone)]
struct ServerState {
    uploads: Arc<DashMap<MediaId, MediaUpload>>,
    // this is fine probably
    sushi: Sender<MessageServer>,
    // channel_user: Arc<DashMap<UserId, (Sender<MessageServer>, Receiver<MessageServer>)>>,
    pool: PgPool,
    blobs: opendal::Operator,
}

impl ServerState {
    fn new(pool: PgPool, blobs: opendal::Operator) -> Self {
        Self {
            uploads: Arc::new(DashMap::new()),
            pool,
            sushi: tokio::sync::broadcast::channel(100).0,
            // channel_user: Arc::new(DashMap::new()),
            blobs,
        }
    }

    fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres {
            pool: self.pool.clone(),
        })
    }

    fn services(&self) -> Services {
        Services::new(self.data())
    }

    fn blobs(&self) -> &opendal::Operator {
        &self.blobs
    }

    async fn presign(&self, url: &str) -> Result<String> {
        // Ok(self
        //     .blobs
        //     .presign_read(&media_id.to_string(), Duration::from_secs(60 * 60 * 24))
        //     .await?
        //     .uri()
        //     .to_string())
        // HACK: temporary thing for better caching
        // TODO: i should use serviceworkers to cache while ignoring signature params
        Ok(format!("https://chat-files.celery.eu.org/{url}"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&std::env::var("DATABASE_URL").expect("missing env var"))
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket(&std::env::var("S3_BUCKET").expect("missing env var"))
        .endpoint(&std::env::var("S3_ENDPOINT").expect("missing env var"))
        .region(&std::env::var("S3_REGION").expect("missing env var"))
        .access_key_id(&std::env::var("S3_ACCESS_KEY_ID").expect("missing env var"))
        .secret_access_key(&std::env::var("S3_SECRET_ACCESS_KEY").expect("missing env var"));
    let blobs = opendal::Operator::new(blobs_builder).unwrap().finish();

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routes::routes())
        .with_state(ServerState::new(pool, blobs))
        .split_for_parts();
    let api1 = api.clone();
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(Scalar::with_url("/scalar", api1).to_html()) }),
        )
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
