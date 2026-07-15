use std::str::FromStr;

use anyhow::Result;
use figment::providers::{Env, Format, Toml};
use lamprey_backend_core::config::Config;
use lamprey_voice::Sfu;
use opentelemetry_otlp::WithExportConfig;
use tracing::debug;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Toml::file("sfu.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    setup_otel(&config)?;

    debug!("hello, world!");

    // let _config_voice = config
    //     .voice
    //     .as_ref()
    //     .expect("missing voice field in config; cannot start sfu");

    // // TODO: validate network interfaces before starting
    // if let Err(e) = util::select_host_address_ipv4(config_voice.host_ipv4.as_deref()) {
    //     error!(
    //         "IPv4 configuration error: {}. A usable IPv4 interface is required.",
    //         e
    //     );
    //     process::exit(1);
    // }

    // if let Err(e) = util::select_host_address_ipv6(config_voice.host_ipv6.as_deref()) {
    //     error!(
    //         "IPv6 configuration error: {}. A usable IPv6 interface is required.",
    //         e
    //     );
    //     process::exit(1);
    // }

    let _handle = Sfu::serve(config).await;

    // TODO: proper shutdown handling
    futures::future::pending::<()>().await;

    Ok(())
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
