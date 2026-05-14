use std::str::FromStr;

use axum::{response::Html, routing::get, Json};
use http::HeaderName;
use opentelemetry_otlp::WithExportConfig;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, propagate_header::PropagateHeaderLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::{config::Config, routes, state::AppState, Result};

pub struct MediaServer {
    state: AppState,
    router: axum::Router,
}
// let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
// axum::serve(listener, router).await?;

impl MediaServer {
    /// setup a server
    pub async fn init_from_config(config: Config) -> Result<Self> {
        let state = AppState::init_from_config(config).await?;
        let server = Self::init(state).await;
        Ok(server)
    }

    pub async fn init(state: AppState) -> Self {
        let router = create_router(state.clone());
        Self { state, router }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub async fn serve(&self) -> Result<()> {
        info!("starting server");

        let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
        axum::serve(listener, self.router.clone()).await?;

        Ok(())
    }
}

pub fn setup_otel(config: &Config) -> Result<()> {
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

    Ok(())
}

#[derive(OpenApi)]
#[openapi(info(title = "cdn docs", description = "documentation for the cdn",))]
struct ApiDoc;

// TODO: use Arc<AppState> instead of AppState
pub fn create_router(state: AppState) -> axum::Router {
    let (router, api): (axum::Router, _) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::routes())
        .with_state(state)
        .split_for_parts();
    router
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
        )))
}

fn cors() -> CorsLayer {
    use http::header::{HeaderName, AUTHORIZATION, CONTENT_TYPE};
    const UPLOAD_OFFSET: HeaderName = HeaderName::from_static("upload-offset");
    const UPLOAD_LENGTH: HeaderName = HeaderName::from_static("upload-length");
    CorsLayer::very_permissive()
        .expose_headers([CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
}
