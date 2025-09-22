use anyhow::Result;
use common::{BridgeMessage, Config, Globals};
use dashmap::DashMap;
use data::Data;
use discord::Discord;
use figment::providers::{Env, Format, Toml};
use lamprey::Lamprey;
use opentelemetry_otlp::WithExportConfig;
use portal::Portal;
use std::{str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{error, info, Instrument};
mod discord;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::common::PortalConfig;

mod common;
mod data;
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
    let (bridge_chan_tx, mut bridge_chan_rx) = mpsc::channel(100);

    let globals = Arc::new(Globals {
        pool,
        config,
        portals: Arc::new(DashMap::new()),
        last_ids: Arc::new(DashMap::new()),
        dc_chan: dc_chan.0,
        ch_chan: ch_chan.0,
        bridge_chan: bridge_chan_tx,
    });

    for config in globals.get_portals().await? {
        let last_id = globals
            .get_last_message_ch(config.lamprey_thread_id)
            .await?;
        if let Some(last_id) = last_id {
            globals.last_ids.insert(config.lamprey_thread_id, last_id);
        }
    }

    let dc = Discord::new(globals.clone(), dc_chan.1);
    let ch = Lamprey::new(globals.clone(), ch_chan.1);

    let bridge_globals = globals.clone();
    let bridge_task = tokio::spawn(async move {
        while let Some(msg) = bridge_chan_rx.recv().await {
            let span = tracing::debug_span!("handle bridge message", ?msg);
            async {
                match msg {
                    BridgeMessage::LampreyThreadCreate {
                        thread_id,
                        room_id,
                        thread_name,
                        discord_guild_id,
                    } => {
                        if bridge_globals
                            .get_portal_by_thread_id(thread_id)
                            .await
                            .is_ok_and(|a| a.is_some())
                        {
                            info!("already exists");
                            return;
                        }

                        info!("autobridging thread {}", thread_id);
                        let name = if thread_name.is_empty() {
                            "thread".to_string()
                        } else {
                            thread_name
                        };
                        let channel_id = match discord::discord_create_channel(
                            bridge_globals.clone(),
                            discord_guild_id,
                            name.clone(),
                        )
                        .await
                        {
                            Ok(channel_id) => channel_id,
                            Err(e) => {
                                error!("failed to create discord channel: {e}");
                                return;
                            }
                        };
                        let webhook = match discord::discord_create_webhook(
                            bridge_globals.clone(),
                            channel_id,
                            "bridge".to_string(),
                        )
                        .await
                        {
                            Ok(webhook) => webhook,
                            Err(e) => {
                                error!("failed to create discord webhook: {e}");
                                return;
                            }
                        };
                        let portal = PortalConfig {
                            lamprey_thread_id: thread_id,
                            lamprey_room_id: room_id,
                            discord_guild_id,
                            discord_channel_id: channel_id,
                            discord_thread_id: None,
                            discord_webhook: webhook.url().unwrap().to_string(),
                        };
                        if let Err(e) = bridge_globals.insert_portal(portal.clone()).await {
                            error!("failed to insert portal: {e}");
                            return;
                        }
                        bridge_globals
                            .portals
                            .entry(portal.lamprey_thread_id)
                            .or_insert_with(|| Portal::summon(bridge_globals.clone(), portal));
                    }
                    BridgeMessage::DiscordChannelCreate {
                        guild_id,
                        channel_id,
                        channel_name,
                    } => {
                        let Ok(realms) = bridge_globals.get_realms().await else {
                            return;
                        };

                        let Some(realm_config) =
                            realms.iter().find(|c| c.discord_guild_id == guild_id)
                        else {
                            return;
                        };

                        if !realm_config.continuous {
                            return;
                        }

                        if bridge_globals
                            .get_portal_by_discord_channel(channel_id)
                            .await
                            .is_ok_and(|a| a.is_some())
                        {
                            info!("already exists");
                            return;
                        }

                        info!("autobridging discord channel {}", channel_id);
                        let ly = match bridge_globals.lamprey_handle().await {
                            Ok(ly) => ly,
                            Err(e) => {
                                error!("failed to get lamprey handle: {e}");
                                return;
                            }
                        };

                        let thread_name = if channel_name.is_empty() {
                            "thread".to_string()
                        } else {
                            channel_name.clone()
                        };

                        let thread = match ly
                            .create_thread(realm_config.lamprey_room_id, thread_name.clone(), None)
                            .await
                        {
                            Ok(thread) => thread,
                            Err(e) => {
                                error!("failed to create lamprey thread: {e}");
                                return;
                            }
                        };

                        let webhook = match discord::discord_create_webhook(
                            bridge_globals.clone(),
                            channel_id,
                            "bridge".to_string(),
                        )
                        .await
                        {
                            Ok(webhook) => webhook,
                            Err(e) => {
                                error!("failed to create discord webhook: {e}");
                                return;
                            }
                        };

                        let portal_config = PortalConfig {
                            lamprey_thread_id: thread.id,
                            lamprey_room_id: realm_config.lamprey_room_id,
                            discord_guild_id: guild_id,
                            discord_channel_id: channel_id,
                            discord_thread_id: None,
                            discord_webhook: webhook.url().unwrap().to_string(),
                        };

                        if let Err(e) = bridge_globals.insert_portal(portal_config.clone()).await {
                            error!(
                                "failed to insert portal for discord channel {}: {e}",
                                channel_id
                            );
                            return;
                        }

                        bridge_globals
                            .portals
                            .entry(portal_config.lamprey_thread_id)
                            .or_insert_with(|| {
                                Portal::summon(bridge_globals.clone(), portal_config)
                            });
                    }
                }
            }
            .instrument(span)
            .await;
        }
    });

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
                        thread_id: thread.id,
                        room_id: realm.lamprey_room_id,
                        thread_name: thread.name,
                        discord_guild_id: realm.discord_guild_id,
                    })
                    .await
                {
                    error!("failed to send lamprey thread create message: {e}");
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    let _ = tokio::join!(
        dc.connect(),
        ch.connect(),
        bridge_task,
        startup_autobridge_task
    );

    Ok(())
}
