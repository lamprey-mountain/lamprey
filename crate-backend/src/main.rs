// TEMP: will remove deprecated routes later
#![allow(deprecated)]

use std::{str::FromStr, sync::Arc, time::Duration};

use axum::{extract::DefaultBodyLimit, response::Html, routing::get, Json};
use clap::Parser;
use common::v1::types::{email::EmailAddr, notifications::InboxFilters};
use figment::providers::{Env, Format, Toml};
use http::{header, HeaderName};
use opendal::layers::LoggingLayer;
use opentelemetry_otlp::WithExportConfig;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, propagate_header::PropagateHeaderLayer,
    sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use utoipa::{openapi::extensions::Extensions, Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use backend::{
    cli, config, error,
    routes::{self},
    types::{self, MessageId, MessageSync, PaginationQuery},
    ServerState,
};

use config::Config;
use error::Result;

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        types::Room,
        types::RoomPatch,
        types::User,
        types::Thread,
        types::ThreadPatch,
        types::Message,
        types::RoomMember,
        types::Role,
        // utoipa seems to forget to add these types specifically
        types::UserIdReq,
        InboxFilters,
        MessageSync,
        PaginationQuery<MessageId>,
    )),
    info(
        title = "api doccery",
        description = "thems the docs",
    ),
    tags(
        (name = "sync", description = include_str!("../docs/sync.md")),
        (name = "auth", description = include_str!("../docs/auth.md")),
    ),
    modifiers(&NestedTags),
)]
struct ApiDoc;

struct NestedTags;

impl Modify for NestedTags {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let tag_groups = json!([
            {
                "name": "auth",
                "description": "authentication and session management",
                "tags": ["session", "auth"],
            },
            {
                "name": "room",
                "description": "working with rooms",
                "tags": ["room", "room_member", "role", "emoji", "tag"],
            },
            {
                "name": "thread",
                "description": "working with threads",
                "tags": ["thread", "thread_member", "message", "reaction", "voice"],
            },
            {
                "name": "user",
                "description": "working with users",
                "tags": ["user", "user_email", "relationship", "dm"],
            },
            {
                "name": "misc",
                "description": "random other routes that i dont have anywhere to put yet",
                "tags": ["debug", "invite", "media", "moderation", "notification", "sync", "search", "application"],
            },
        ]);
        openapi
            .extensions
            .get_or_insert_default()
            .merge(Extensions::builder().add("x-tagGroups", tag_groups).build());
    }
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
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let args = cli::Args::parse();

    let config: Config = figment::Figment::new()
        .merge(Toml::file(args.config))
        // .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    let mut exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary);
    if let Some(endpoint) = &config.otel_trace_endpoint {
        exporter = exporter.with_endpoint(endpoint);
    }
    let exporter = exporter.build()?;
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    use opentelemetry::trace::TracerProvider;
    let tracer = provider.tracer("backend");
    opentelemetry::global::set_tracer_provider(provider);

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(EnvFilter::from_str(&config.rust_log)?)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    match &args.command {
        cli::Command::Serve {} => serve(config).await?,
        cli::Command::Check {} => check(config).await?,
        cli::Command::TestMail { to } => send_test_email(config, to.to_owned()).await?,
    }

    Ok(())
}

/// start the main server
async fn serve(config: Config) -> Result<()> {
    info!("Starting server with config: {:#?}", config);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(config.s3.endpoint.as_str())
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);
    let blobs = opendal::Operator::new(blobs_builder)?
        .layer(LoggingLayer::default())
        .finish();
    blobs.check().await?;

    let state = Arc::new(ServerState::new(config, pool, blobs));

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routes::routes())
        .with_state(state)
        .split_for_parts();
    let router = router
        .route("/api/docs.json", get(|| async { Json(api) }))
        .route(
            "/api/docs",
            get(|| async { Html(include_str!("scalar.html")) }),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024 * 16))
        .layer(cors())
        .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-trace-id",
        )));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    axum::serve(listener, router).await?;
    Ok(())
}

/// check config
async fn check(config: Config) -> Result<()> {
    info!("Parsed config: {:#?}", config);
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(config.s3.endpoint.as_str())
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);
    let blobs = opendal::Operator::new(blobs_builder)?
        .layer(LoggingLayer::default())
        .finish();
    blobs.check().await?;
    Ok(())
}

async fn send_test_email(config: Config, to: String) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let blobs_builder = opendal::services::S3::default()
        .bucket(&config.s3.bucket)
        .endpoint(config.s3.endpoint.as_str())
        .region(&config.s3.region)
        .access_key_id(&config.s3.access_key_id)
        .secret_access_key(&config.s3.secret_access_key);
    let blobs = opendal::Operator::new(blobs_builder)?
        .layer(LoggingLayer::default())
        .finish();
    blobs.check().await?;

    let state = Arc::new(ServerState::new(config, pool, blobs));
    state.services().email.test().await?;
    state
        .services()
        .email
        .send(
            EmailAddr::new(to)?,
            "test email".to_owned(),
            "test email, please ignore".to_owned(),
        )
        .await?;
    Ok(())
}
