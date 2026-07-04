use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use lamprey_media::{
    config::Config,
    server::{MediaServer, setup_otel},
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let config: Config = Figment::new()
        .merge(Toml::file("cdn.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    setup_otel(&config)?;

    info!("starting cdn with config: {:#?}", config);

    let server = MediaServer::init_from_config(config).await?;
    server.serve().await?;

    Ok(())
}
