use std::str::FromStr;

use anyhow::Result;
use figment::providers::{Env, Format, Toml};
use lamprey_voice::{config::Config, sfu::Sfu};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = figment::Figment::new()
        .merge(Toml::file("sfu.toml"))
        .merge(Env::raw())
        .extract()?;

    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str(&config.rust_log)?)
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    let _ = Sfu::run(config.clone()).await;

    Ok(())
}
