use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json,
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;
use tracing_subscriber::EnvFilter;
use voice::{sfu::Sfu, SfuCommand};

async fn start_http(wheel: UnboundedSender<SfuCommand>) -> Result<()> {
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
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    let wheel = Sfu::default().spawn();
    let _ = start_http(wheel).await;

    Ok(())
}
