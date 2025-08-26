use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use common::v1::types::{EmojiId, MediaId};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use http::{header, HeaderName};
use opendal::{layers::LoggingLayer, Operator};
use serde::Deserialize;
use sqlx::query_scalar;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{io::Cursor, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, propagate_header::PropagateHeaderLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

use crate::{
    config::Config,
    error::{Error, Result},
};

mod config;
mod data;
mod error;
mod routes;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    s3: Operator,
    config: Arc<Config>,
}

fn cors() -> CorsLayer {
    use header::{HeaderName, AUTHORIZATION, CONTENT_TYPE};
    const UPLOAD_OFFSET: HeaderName = HeaderName::from_static("upload-offset");
    const UPLOAD_LENGTH: HeaderName = HeaderName::from_static("upload-length");
    CorsLayer::very_permissive()
        .expose_headers([CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config: Config = Figment::new()
        .merge(Toml::file("cdn.toml"))
        .merge(Env::raw())
        .extract()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str(&config.rust_log)?)
        .init();

    info!("starting cdn with config: {:#?}", config);

    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    let builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(config.s3.endpoint.as_str())
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);
    let s3 = Operator::new(builder)?
        .layer(LoggingLayer::default())
        .finish();

    let state = AppState {
        db,
        s3,
        config: Arc::new(config),
    };

    // let state = Arc::new(ServerState::new(config, pool, blobs));

    // let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
    //     .nest("/api/v1", routes::routes())
    //     .with_state(state)
    //     .split_for_parts();
    // let router = router
    //     .route("/api/docs.json", get(|| async { Json(api) }))
    //     .route(
    //         "/api/docs",
    //         get(|| async { Html(include_str!("scalar.html")) }),
    //     )

    let app = Router::new()
        .merge(routes::routes())
        .with_state(state)
        .route("/", get(|| async { "it works!" }))
        .layer(cors())
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
