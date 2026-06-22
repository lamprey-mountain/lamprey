use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use lamprey_bridge::{
    bridge::{BridgeEvent, BridgeHandle},
    config::{Config, ConfigPlatform},
    database::SqliteDatabase,
    discord, lamprey,
};
use opentelemetry_otlp::WithExportConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::{str::FromStr, sync::Arc};
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    info!(?config, "loaded config");

    // set up logging/tracing
    if let Some(endpoint) = &config.otel_trace_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        use opentelemetry::trace::TracerProvider;
        let tracer = provider.tracer("bridge");
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

    // connect to db
    let options = SqliteConnectOptions::from_str(&config.database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let db = SqliteDatabase::new(pool);

    // spawn connections to platforms
    let mut readys = vec![];
    let mut tasks = vec![];
    let bridge = BridgeHandle::new(Arc::new(db));

    for (_name, s) in &config.platform {
        let p = match s {
            ConfigPlatform::Lamprey(c) => lamprey::spawn(bridge.clone(), c.clone()),
            ConfigPlatform::Discord(c) => discord::spawn(bridge.clone(), c.clone()),
        };

        readys.push(p.ready);
        tasks.push((p.name, p.task));
    }

    for ready in readys {
        ready.await.unwrap();
    }

    // init realms and portals
    let realms = bridge.db.realm_list().await?;
    let portals = bridge.db.portal_list().await?;

    for (id, realm) in realms {
        // TODO: create realms
    }

    for (id, portal) in portals {
        let handle = bridge.create_portal_handle(id);
        let event = BridgeEvent::PortalInit(id, portal, handle);
        bridge
            .events
            .send(Arc::new(event))
            .expect("TODO: better error handling");
    }

    // supervise everything
    let mut taskset = JoinSet::new();
    for (name, task) in tasks {
        taskset.spawn(async move {
            let result = task.await;
            (name, result)
        });
    }

    while let Some(result) = taskset.join_next().await {
        let (name, res) = result?;
        match res {
            Ok(_) => warn!(name, "platform exited cleanly"),
            Err(err) => error!(name, ?err, "platform crashed"),
            // TODO: restart logic here
        }
    }

    // TODO: proper shutdown handling
    futures::future::pending::<()>().await;

    Ok(())
}
