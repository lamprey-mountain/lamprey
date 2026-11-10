use std::str::FromStr;
use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider, trace::SdkTracerProvider};
use prometheus::Registry;
use tracing_subscriber::{EnvFilter, Registry as SubscriberRegistry, layer::SubscriberExt};

use crate::{config::Config, prelude::*};

static LOGGER_PROVIDER: OnceLock<SdkLoggerProvider> = OnceLock::new();
static PROMETHEUS_REGISTRY: OnceLock<Registry> = OnceLock::new();

/// the prometheus registry that all opentelemetry metrics are exported to
pub fn prometheus_registry() -> Option<&'static Registry> {
    PROMETHEUS_REGISTRY.get()
}

/// setup tracing, opentelemetry, etc based on a server config
pub fn init(config: &Config) -> ServerResult<()> {
    let filter = EnvFilter::from_str(&config.rust_log)?;
    let fmt_layer = tracing_subscriber::fmt::layer();

    let resource = Resource::builder()
        .with_service_name("lamprey-backend")
        .build();

    // prometheus is always available so /metrics does not depend on otlp being enabled
    let registry = Registry::new();
    let prometheus_exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()
        .map_err(ServerError::internal)?;
    let _ = PROMETHEUS_REGISTRY.set(registry);

    let mut meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_reader(prometheus_exporter);

    if config.otel.enabled {
        let mut exporter_builder = opentelemetry_otlp::MetricExporter::builder().with_tonic();
        if let Some(endpoint) = &config.otel.metrics {
            exporter_builder = exporter_builder.with_endpoint(endpoint);
        }
        meter_provider = meter_provider
            .with_periodic_exporter(exporter_builder.build().map_err(ServerError::internal)?);
    }

    opentelemetry::global::set_meter_provider(meter_provider.build());

    let (telemetry_layer, log_layer) = if config.otel.enabled {
        let traces = config
            .otel
            .traces
            .as_deref()
            .or(config.otel_trace_endpoint.as_deref());
        let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder().with_tonic();
        if let Some(endpoint) = traces {
            exporter_builder = exporter_builder.with_endpoint(endpoint);
        }
        let provider = SdkTracerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(exporter_builder.build().map_err(ServerError::internal)?)
            .build();
        let tracer = provider.tracer("lamprey-backend");

        // the tracer provider is kept alive by the global tracer provider
        opentelemetry::global::set_tracer_provider(provider);
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let mut exporter_builder = opentelemetry_otlp::LogExporter::builder().with_tonic();
        if let Some(endpoint) = &config.otel.logs {
            exporter_builder = exporter_builder.with_endpoint(endpoint);
        }
        let provider = SdkLoggerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter_builder.build().map_err(ServerError::internal)?)
            .build();
        let log_layer = OpenTelemetryTracingBridge::builder(&provider).build();
        // the logger provider must stay alive for as long as the logger is used
        let _ = LOGGER_PROVIDER.set(provider);

        (Some(telemetry_layer), Some(log_layer))
    } else {
        (None, None)
    };

    let subscriber = SubscriberRegistry::default()
        .with(filter)
        .with(fmt_layer)
        .with(telemetry_layer)
        .with(log_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
