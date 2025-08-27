use axum::{response::Html, routing::get, Json};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use http::{header, HeaderName};
use opendal::{layers::LoggingLayer, Operator};
use opentelemetry_otlp::WithExportConfig;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{str::FromStr, sync::Arc, time::Duration};
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, propagate_header::PropagateHeaderLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::config::Config;

mod config;
mod data;
mod error;
mod routes;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    s3: Operator,
    config: Arc<Config>,
    // pending_thumbnails: Arc<Mutex<HashMap<(MediaId, u64, u64), Arc<Mutex<()>>>>>,
}

#[derive(OpenApi)]
#[openapi(info(title = "cdn docs", description = "documentation for the cdn",))]
struct ApiDoc;

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

    let mut exporter = opentelemetry_otlp::SpanExporter::builder().with_tonic();
    if let Some(endpoint) = &config.otel_trace_endpoint {
        exporter = exporter.with_endpoint(endpoint);
    }
    let exporter = exporter.build()?;
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();
    use opentelemetry::trace::TracerProvider;
    let tracer = provider.tracer("cdn");
    opentelemetry::global::set_tracer_provider(provider);
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default()
        .with(EnvFilter::from_str(&config.rust_log)?)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer);
    tracing::subscriber::set_global_default(subscriber)?;

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

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::routes())
        .with_state(state)
        .split_for_parts();
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("scalar.html")) }),
        )
        .route("/", get(|| async { "it works!" }))
        .layer(cors())
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
    axum::serve(listener, router).await?;

    Ok(())
}
