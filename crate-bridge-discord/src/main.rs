use anyhow::Result;
use common::{Config, Globals};
use dashmap::DashMap;
use data::Data;
use discord::Discord;
use figment::providers::{Env, Format, Toml};
use lamprey::Lamprey;
use opentelemetry_otlp::WithExportConfig;
use std::{str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::bridge::{Bridge, BridgeMessage};

mod bridge;
mod common;
mod data;
mod discord;
mod lamprey;
mod portal;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    info!("config {config:#?}");

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

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let dc_chan = mpsc::channel(100);
    let ch_chan = mpsc::channel(100);
    let bridge_chan = mpsc::unbounded_channel();

    let globals = Arc::new(Globals {
        pool,
        config,
        portals: Arc::new(DashMap::new()),
        last_ids: Arc::new(DashMap::new()),
        presences: Arc::new(DashMap::new()),
        dc_chan: dc_chan.0,
        ch_chan: ch_chan.0,
        bridge_chan: bridge_chan.0,
    });

    for config in globals.get_portals().await? {
        let last_id = globals
            .get_last_message_ch(config.lamprey_thread_id)
            .await?;
        if let Some(last_id) = last_id {
            globals.last_ids.insert(config.lamprey_thread_id, last_id);
        }
    }

    let presence_globals = globals.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            info!("Re-syncing all user presences");
            for item in presence_globals.presences.iter() {
                let presence = item.value().clone();
                let globals = presence_globals.clone();
                tokio::spawn(async move {
                    if let Err(e) = discord::process_presence_update(globals, presence).await {
                        error!("failed to re-sync presence: {e}");
                    }
                });
            }
        }
    });

    let dc = Discord::new(globals.clone(), dc_chan.1);
    let ch = Lamprey::new(globals.clone(), ch_chan.1);
    Bridge::spawn(globals.clone(), bridge_chan.1);

    let startup_autobridge_task = tokio::spawn(async move {
        let globals = globals.clone();
        for realm in globals.get_realms().await? {
            if !realm.continuous {
                continue;
            }

            info!("creating new portal for {:?}", realm);
            let ly = globals.lamprey_handle().await?;
            let threads = ly.room_threads(realm.lamprey_room_id).await?;
            for thread in threads {
                if globals.get_portal_by_thread_id(thread.id).await?.is_some() {
                    continue;
                }
                if let Err(e) = globals
                    .bridge_chan
                    .send(BridgeMessage::LampreyThreadCreate {
                        thread,
                        discord_guild_id: realm.discord_guild_id,
                    })
                {
                    error!("failed to send lamprey thread create message: {e}");
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    let _ = tokio::join!(dc.connect(), ch.connect(), startup_autobridge_task);

    Ok(())
}
