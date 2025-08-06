use std::sync::Arc;

use anyhow::Result;
use common::{Config, Globals};
use dashmap::DashMap;
use data::Data;
use discord::Discord;
use figment::providers::{Env, Format, Toml};
use lamprey::Lamprey;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

mod common;
mod data;
mod discord;
mod lamprey;
mod portal;

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

    for config in &globals.config.portal {
        let last_id = globals.get_last_message_ch(config.my_thread_id).await?;
        if let Some(last_id) = last_id {
            globals.last_ids.insert(config.my_thread_id, last_id);
        }
    }

    let dc = Discord::new(globals.clone(), dc_chan.1);
    let ch = Lamprey::new(globals.clone(), ch_chan.1);

    let _ = tokio::join!(dc.connect(), ch.connect());

    Ok(())
}
