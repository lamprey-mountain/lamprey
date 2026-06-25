use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use kerosene_rest::Routes;
use lamprey_backend_core::{config::Config, prelude::*};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::raw())
        .extract()?;

    info!("booting up with config: {:#?}", config);

    let globals = todo!();

    let router = Routes::new_api()
        .into_axum()
        // #[cfg(feature = "embed-frontend")]
        // .layer(frontend) // or .nest? or .fallback?
        // .layer(one)
        // .layer(two)
        // .layer(three)
        .with_state(globals);
    // axum::serve(listener, router)

    // let server = Server::init_from_config(config).await?;
    // server.serve().await?;

    // TODO: copy crate-backend/src/serve/mod.rs

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
