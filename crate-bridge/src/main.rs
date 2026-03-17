use ::common::v1::types::{PaginationDirection, PaginationQuery};
use anyhow::Result;
use bridge_common::Globals;
use db::Data;
use discord::Discord;
use figment::providers::{Env, Format, Toml};
use kameo::actor::Spawn;
use lamprey::{Lamprey, LampreyMessage, LampreyResponse};
use opentelemetry_otlp::WithExportConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
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
mod db;
mod discord;
mod lamprey;
mod mentions;
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

    let options = SqliteConnectOptions::from_str(&config.database_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // Create globals with empty actor refs
    let globals = Arc::new(Globals::new(pool, config));

    // supervisor for bridge actor
    let supervisor_globals = globals.clone();
    tokio::spawn(async move {
        loop {
            tracing::info!("Starting Bridge Actor...");
            let bridge_ref = Bridge::spawn((supervisor_globals.clone(),));
            supervisor_globals.set_bridge_chan(bridge_ref.clone()).await;
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;

                // HACK: check globals reference to see if actor is alive
                // this should be a no-op message instead
                let current = supervisor_globals.bridge_chan.read().await;
                if let Some(ref current_ref) = *current {
                    if !current_ref.eq(&bridge_ref) {
                        // actor was replaced
                        drop(current);
                        break;
                    }
                } else {
                    // actor was cleared
                    drop(current);
                    break;
                }
                drop(current);
            }

            error!("Bridge Actor crashed or stopped! Restarting in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // supervisor for lamprey actor
    let supervisor_globals = globals.clone();
    tokio::spawn(async move {
        loop {
            tracing::info!("Starting Lamprey Actor...");
            let lamprey_ref = Lamprey::spawn(supervisor_globals.clone());
            supervisor_globals
                .set_lamprey_chan(lamprey_ref.clone())
                .await;

            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;

                let current = supervisor_globals.lamprey_chan.read().await;
                if let Some(ref current_ref) = *current {
                    if !current_ref.eq(&lamprey_ref) {
                        // actor was replaced
                        drop(current);
                        break;
                    }
                } else {
                    // actor was cleared
                    drop(current);
                    break;
                }
                drop(current);
            }

            error!("Lamprey Actor crashed or stopped! Restarting in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Create discord actor (not spawned via kameo - serenity runs its own event loop)
    // We need to create it, store a reference in globals, then clone it for connect()
    let discord = Discord::new(globals.clone());
    globals.set_discord(discord.clone())?;

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
    let presence_semaphore = Arc::new(Semaphore::new(5)); // Max 5 concurrent presence syncs
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(120)).await;
            info!("Re-syncing all user presences");

            // Collect presences first to avoid holding DashMap lock while awaiting semaphore
            let presences: Vec<_> = presence_globals
                .presences
                .iter()
                .map(|item| item.value().clone())
                .collect();

            for presence in presences {
                let globals = presence_globals.clone();
                let semaphore = presence_semaphore.clone();
                tokio::spawn(async move {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    let _permit = permit; // hold permit for duration of task
                    if let Err(e) = discord::process_presence_update(globals, presence).await {
                        error!("failed to re-sync presence: {e}");
                    }
                });
            }
        }
    });

    // Spawn a task to run the lamprey syncer
    let lamprey_syncer_task = tokio::spawn(async move {
        // The lamprey actor is already running via kameo
        // We need to keep the process alive for the syncer
        // The syncer runs inside the actor, so we just need to wait
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let startup_autobridge_globals = globals.clone();
    let _startup_autobridge_task = tokio::spawn(async move {
        let globals = startup_autobridge_globals;

        // Wait for lamprey actor to be initialized
        let lamprey_ref = globals.wait_for_lamprey().await?;

        for realm in globals.get_realms().await? {
            if !realm.continuous {
                continue;
            }

            info!("creating new portal for {:?}", realm);

            // Use kameo's ask pattern to get room threads
            let threads_response = lamprey_ref
                .ask(LampreyMessage::RoomThreads {
                    room_id: realm.lamprey_room_id,
                })
                .await?;

            let threads = match threads_response {
                LampreyResponse::RoomThreads(t) => t,
                _ => return Err(anyhow::anyhow!("unexpected response type")),
            };

            for thread in threads {
                if globals.get_portal_by_thread_id(thread.id).await?.is_some() {
                    continue;
                }
                globals
                    .bridge_send(BridgeMessage::LampreyThreadCreate {
                        thread,
                        discord_guild_id: realm.discord_guild_id,
                    })
                    .await?;
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    let lamprey_backfill_globals = globals.clone();
    let _lamprey_backfill_task = tokio::spawn(async move {
        let globals = lamprey_backfill_globals;
        info!("starting lamprey backfill");
        let portals = globals.get_portals().await?;

        let backfill_semaphore = Arc::new(Semaphore::new(5)); // Max 5 concurrent backfills

        for portal_config in portals {
            let globals = globals.clone();
            let permit = backfill_semaphore.clone().acquire_owned().await.unwrap();
            tokio::spawn(async move {
                let _permit = permit; // hold permit for duration of task
                let res: Result<()> = async {
                    // Wait for lamprey actor to be initialized
                    let lamprey_ref = globals.wait_for_lamprey().await?;

                    let from = globals
                        .last_lamprey_ids
                        .get(&portal_config.lamprey_thread_id)
                        .map(|m| *m.value());
                    let from_start = from;
                    let mut current_from = from_start;

                    loop {
                        let query = PaginationQuery {
                            from: current_from,
                            to: None,
                            dir: Some(PaginationDirection::F),
                            limit: Some(100),
                        };

                        let page_response = lamprey_ref
                            .ask(LampreyMessage::MessageList {
                                thread_id: portal_config.lamprey_thread_id,
                                query: Arc::new(query),
                            })
                            .await?;

                        let page = match page_response {
                            LampreyResponse::MessageList(p) => p,
                            _ => return Err(anyhow::anyhow!("unexpected response type")),
                        };

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
                            current_from = Some(cursor.parse()?);
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

    // Run the actors using select! for proper error handling
    // discord.connect() is the main long-running future (serenity event loop)
    // Background tasks run concurrently and are cancelled if discord.connect() fails
    tokio::select! {
        dc_res = discord.connect() => {
            error!("Discord connection ended: {:?}", dc_res);
            dc_res?
        }
        res = lamprey_syncer_task => {
            error!("Lamprey syncer task failed: {:?}", res);
            res?;
        }
    }

    Ok(())
}
