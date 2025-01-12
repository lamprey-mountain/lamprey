use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    response::Html, routing::get, Json
};
use dashmap::DashMap;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_subscriber::EnvFilter;
use types::{MediaId, MediaUpload};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable as _};

// pub mod data;
pub mod types;
pub mod error;
mod routes;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        types::Room,
        types::RoomPatch,
        types::User,
        types::Thread,
        types::ThreadPatch,
        types::Message,
        types::Member,
        types::Role,
    ))
)]
struct ApiDoc;

#[derive(Clone)]
struct ServerState {
    uploads: Arc<DashMap<MediaId, MediaUpload>>,
    // channel_thread: Arc<DashMap<ThreadId, (Sender, Reciever)>>,
    // channel_room: Arc<DashMap<RoomId, (Sender, Reciever)>>,
    // channel_user: Arc<DashMap<UserId, (Sender, Reciever)>>,
    pool: PgPool,
}

impl ServerState {
    fn new(pool: PgPool) -> Self {
        Self {
            uploads: Arc::new(DashMap::new()),
            pool,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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
    
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routes::routes())
        .with_state(ServerState::new(pool))
        .split_for_parts();
    let api1 = api.clone();
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route("/api/docs", get(|| async { Html(Scalar::with_url("/scalar", api1).to_html()) }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
