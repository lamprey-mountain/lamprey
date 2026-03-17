use axum::{response::Html, routing::get, Json};
use common::{
    v1::types::{EmojiId, MediaId, MediaV0, MessageSync},
    v2::types::media::MediaStatus,
};
use error::Result;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use http::{header, HeaderName};
use moka::future::Cache;
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

use crate::{config::Config, error::Error};

mod config;
mod data;
mod error;
mod ffmpeg;
mod routes;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    s3: Operator,
    nats: Option<async_nats::Client>,
    config: Arc<Config>,

    // NOTE: be careful about allowing emoji/media editing! i'd need to invalidate these caches
    cache_emoji: Cache<EmojiId, MediaId>,
    cache_media: Cache<MediaId, MediaV0>,
    pending_thumbnails: Cache<(MediaId, u32, u32, bool), Vec<u8>>,
    pending_gifv: Cache<MediaId, Arc<async_tempfile::TempFile>>,

    sushi_tx: tokio::sync::broadcast::Sender<MessageSync>,
}

impl AppState {
    async fn lookup_emoji(&self, emoji_id: EmojiId) -> Result<MediaId> {
        if let Some(m) = self.cache_emoji.get(&emoji_id).await {
            return Ok(m);
        }
        let m = data::lookup_emoji(&self.db, emoji_id).await?;
        self.cache_emoji.insert(emoji_id, m).await;
        Ok(m)
    }

    async fn ensure_media_ready(&self, media_id: MediaId, wait: bool) -> Result<MediaV0> {
        if let Some(m) = self.cache_media.get(&media_id).await {
            return Ok(m);
        }

        let mut sub = self.sushi_tx.subscribe();

        loop {
            let (media, status) = data::lookup_media_with_status(&self.db, media_id).await?;
            if matches!(
                status,
                Some(MediaStatus::Uploaded) | Some(MediaStatus::Consumed) | None
            ) {
                self.cache_media.insert(media_id, media.clone()).await;
                return Ok(media);
            }

            if !wait {
                return Err(Error::StillProcessing);
            }

            if self.nats.is_some() {
                loop {
                    match sub.recv().await {
                        Ok(MessageSync::MediaProcessed { media: m, .. }) if m.id == media_id => {
                            let media_v1: MediaV0 = data::DbMediaData::V2(m).into();
                            self.cache_media.insert(media_id, media_v1.clone()).await;
                            return Ok(media_v1);
                        }
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            break; // Re-check DB
                        }
                        Err(_) => {
                            return Err(Error::Internal(
                                "NATS subscription ended unexpectedly".to_string(),
                            ));
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
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

    if let Some(endpoint) = &config.otel_trace_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        use opentelemetry::trace::TracerProvider;
        let tracer = provider.tracer("bridge-discord");
        opentelemetry::global::set_tracer_provider(provider);
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default()
            .with(EnvFilter::from_str(&config.rust_log)?)
            .with(tracing_subscriber::fmt::layer())
            .with(telemetry_layer);
        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        let subscriber = Registry::default()
            .with(EnvFilter::from_str(&config.rust_log)?)
            .with(tracing_subscriber::fmt::layer());
        tracing::subscriber::set_global_default(subscriber)?;
    }

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

    let nats = if let Some(nats_config) = &config.nats {
        let mut nats_options = async_nats::ConnectOptions::new();
        if let Some(credentials_path) = &nats_config.credentials {
            nats_options = nats_options
                .credentials_file(credentials_path)
                .await
                .map_err(|e| anyhow::anyhow!("NATS credentials file failed: {}", e))?;
        }
        Some(
            async_nats::connect_with_options(&nats_config.addr, nats_options)
                .await
                .map_err(|e| anyhow::anyhow!("NATS connect failed: {}", e))?,
        )
    } else {
        None
    };

    let (sushi_tx, _) = tokio::sync::broadcast::channel(100);

    if let Some(nats_client) = &nats {
        let nats_client = nats_client.clone();
        let sushi_tx = sushi_tx.clone();
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut sub = match nats_client.subscribe("sushi").await {
                Ok(sub) => sub,
                Err(e) => {
                    tracing::error!("failed to subscribe to NATS: {}", e);
                    return;
                }
            };

            while let Some(msg) = sub.next().await {
                if let Ok(sync_msg) = serde_json::from_slice::<MessageSync>(&msg.payload) {
                    let _ = sushi_tx.send(sync_msg);
                }
            }
        });
    }

    let cache_media = Cache::new(config.cache_media);
    let cache_emoji = Cache::new(config.cache_emoji);
    let state = AppState {
        db,
        s3,
        nats,
        config: Arc::new(config),
        cache_media,
        cache_emoji,
        pending_thumbnails: Cache::new(0),
        pending_gifv: Cache::new(100),
        sushi_tx,
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
