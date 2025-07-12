use std::str::FromStr;

use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json,
};
use figment::providers::{Env, Format, Toml};
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;
use tracing_subscriber::EnvFilter;
use voice::{config::Config, sfu::Sfu, SfuCommand};

async fn start_http(config: &Config, wheel: UnboundedSender<SfuCommand>) -> Result<()> {
    let router = axum::Router::new()
        .route(
            "/rpc",
            post(|Json(req): Json<SfuCommand>| async move {
                // handles events proxied through the websocket
                if let Err(err) = wheel.send(req) {
                    error!("error while sending command: {err}");
                };
                StatusCode::ACCEPTED
            }),
        )
        .route("/ping", get(|| async { StatusCode::NO_CONTENT }));
    let listener = tokio::net::TcpListener::bind(&config.host).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

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

    let wheel = Sfu::new(config.clone()).spawn();
    let _ = start_http(&config, wheel).await;

    Ok(())
}
