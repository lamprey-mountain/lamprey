use std::{sync::Arc, time::Duration};

use axum::{response::Html, routing::get, Json};
use dashmap::DashMap;
use data::{postgres::Postgres, Data};
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

    fn blobs(&self) -> &opendal::Operator {
        &self.blobs
    }

    async fn presign(&self, media_id: MediaId) -> Result<String> {
        Ok(self
            .blobs
            .presign_read(&media_id.to_string(), Duration::from_secs(60 * 60 * 24))
            .await?
            .uri()
            .to_string())
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
        .connect("postgres://chat:ce00eebd05027ca1@localhost:5432/chat")
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket("chat-files")
        .endpoint("https://s4.celery.eu.org")
        .region("garage")
        .access_key_id("GKd087b108e26a93db4bc07ac5")
        .secret_access_key("0447ebcbb6b3e21306a0b278687bd7a6ffcd04097fd3dbd18a5250c92664eeab");
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
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
