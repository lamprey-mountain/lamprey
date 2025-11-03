use ::common::v1::types::{PaginationDirection, PaginationQuery};
use anyhow::Result;
use bridge_common::Globals;
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

use crate::{
    bridge::{Bridge, BridgeMessage},
    bridge_common::GlobalsTrait,
    config::Config,
};

mod bridge;
// TODO: rename this to avoid conflicts
mod bridge_common;
mod config;
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
        last_lamprey_ids: Arc::new(DashMap::new()),
        last_discord_ids: Arc::new(DashMap::new()),
        presences: Arc::new(DashMap::new()),
        discord_user_cache: Arc::new(DashMap::new()),
        dc_chan: dc_chan.0,
        ch_chan: ch_chan.0,
        bridge_chan: bridge_chan.0,
    });

    for config in globals.get_portals().await? {
        if let Some(last_id) = globals
            .get_last_message_ch(config.lamprey_thread_id)
            .await?
        {
            globals
                .last_lamprey_ids
                .insert(config.lamprey_thread_id, last_id.chat_id);
        }

        let discord_channel_id = config
            .discord_thread_id
            .unwrap_or(config.discord_channel_id);
        if let Some(last_id) = globals.get_last_message_dc(discord_channel_id).await? {
            globals
                .last_discord_ids
                .insert(discord_channel_id, last_id.discord_id);
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

    let startup_autobridge_globals = globals.clone();
    let startup_autobridge_task = tokio::spawn(async move {
        let globals = startup_autobridge_globals;
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

    let lamprey_backfill_globals = globals.clone();
    let lamprey_backfill_task = tokio::spawn(async move {
        let globals = lamprey_backfill_globals;
        info!("starting lamprey backfill");
        let portals = globals.get_portals().await?;

        for portal_config in portals {
            let globals = globals.clone();
            tokio::spawn(async move {
                let res: Result<()> = async {
                    let ly = globals.lamprey_handle().await?;
                    let from = globals
                        .last_lamprey_ids
                        .get(&portal_config.lamprey_thread_id)
                        .map(|m| *m.value());
                    let mut query = PaginationQuery {
                        from,
                        to: None,
                        dir: Some(PaginationDirection::F),
                        limit: Some(100),
                    };

                    loop {
                        let page = ly
                            .message_list(portal_config.lamprey_thread_id, &query)
                            .await?;
                        info!(
                            "backfilling {} messages for thread {}",
                            page.items.len(),
                            portal_config.lamprey_thread_id
                        );

                        for message in page.items {
                            globals
                                .portal_send(
                                    portal_config.lamprey_thread_id,
                                    portal::PortalMessage::LampreyMessageCreate { message },
                                )
                                .await;
                        }

                        if !page.has_more {
                            break;
                        }

                        if let Some(cursor) = page.cursor {
                            query.from = Some(cursor.parse()?);
                        } else {
                            break;
                        }
                    }
                    Ok(())
                }
                .await;
                if let Err(e) = res {
                    error!(
                        "failed to backfill thread {}: {}",
                        portal_config.lamprey_thread_id, e
                    );
                } else {
                    info!(
                        "finished backfill for thread {}",
                        portal_config.lamprey_thread_id
                    );
                }
            });
        }
        Ok::<(), anyhow::Error>(())
    });

    let _ = tokio::join!(
        dc.connect(),
        ch.connect(),
        startup_autobridge_task,
        lamprey_backfill_task
    );

    Ok(())
}
