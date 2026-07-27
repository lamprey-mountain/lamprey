use std::str::FromStr;

use crate::prelude::*;
use lamprey_backend_core::config::Config;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

/// setup tracing, opentelemetry, etc based on a server config
pub fn init(config: &Config) -> ServerResult<()> {
    if let Some(endpoint) = &config.otel_trace_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        let tracer = provider.tracer("lamprey-api");
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
