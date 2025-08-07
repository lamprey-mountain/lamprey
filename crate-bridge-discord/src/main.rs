use anyhow::Result;
use common::{Config, Globals};
use dashmap::DashMap;
use data::{Data, PortalConfig};
use discord::{Discord, DiscordMessage};
use serenity::all::{GuildId, Webhook};
use tokio::sync::oneshot;
use tracing::info;
mod discord;
use figment::providers::{Env, Format, Toml};
use lamprey::Lamprey;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

mod common;
mod data;
mod lamprey;
mod portal;

async fn discord_create_channel(
    globals: Arc<Globals>,
    guild_id: GuildId,
    name: String,
) -> Result<serenity::all::ChannelId> {
    let (send, recv) = oneshot::channel();
    globals
        .dc_chan
        .send(DiscordMessage::ChannelCreate {
            guild_id,
            name,
            response: send,
        })
        .await?;
    Ok(recv.await?)
}

async fn discord_create_webhook(
    globals: Arc<Globals>,
    channel_id: serenity::all::ChannelId,
    name: String,
) -> Result<Webhook> {
    let (send, recv) = oneshot::channel();
    globals
        .dc_chan
        .send(DiscordMessage::WebhookCreate {
            channel_id,
            name,
            response: send,
        })
        .await?;
    Ok(recv.await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let dc_chan = mpsc::channel(100);
    let ch_chan = mpsc::channel(100);

    let globals = Arc::new(Globals {
        pool,
        config,
        portals: Arc::new(DashMap::new()),
        last_ids: Arc::new(DashMap::new()),
        dc_chan: dc_chan.0,
        ch_chan: ch_chan.0,
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

    let task_autobridge = async {
        for bridge in &globals.config.autobridge {
            info!("autobridging {}", bridge.lamprey_room_id);
            let ly = globals.lamprey_handle().await?;
            info!("a");
            let threads = ly.room_threads(bridge.lamprey_room_id).await?;
            info!("b");
            for thread in threads {
                if globals.get_portal_by_thread_id(thread.id).await?.is_some() {
                    continue;
                }
                info!("autobridging thread {}", thread.id);
                let name = if thread.name.is_empty() {
                    "thread".to_string()
                } else {
                    thread.name
                };
                let channel_id =
                    discord_create_channel(globals.clone(), bridge.discord_guild_id, name.clone())
                        .await?;
                let webhook = discord_create_webhook(globals.clone(), channel_id, name).await?;
                let portal = PortalConfig {
                    lamprey_thread_id: thread.id,
                    lamprey_room_id: bridge.lamprey_room_id,
                    discord_guild_id: bridge.discord_guild_id,
                    discord_channel_id: channel_id,
                    discord_thread_id: None,
                    discord_webhook: webhook.url()?,
                };
                globals.insert_portal(portal).await?;
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    let _ = tokio::join!(dc.connect(), ch.connect(), task_autobridge);

    Ok(())
}
