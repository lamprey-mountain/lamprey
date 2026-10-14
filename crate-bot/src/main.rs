use figment::providers::Format;
use lamprey_bot::{bot::Bot, config::Config};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(figment::providers::Toml::file("bot.toml"))
        .merge(figment::providers::Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    tracing_subscriber::fmt()
        .with_env_filter(&config.rust_log)
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    info!("config {config:#?}");

    let bot = Bot::from_config(config).await?;
    bot.start().await;

    // TODO: proper shutdown handling
    futures::future::pending::<()>().await;

    Ok(())
}
