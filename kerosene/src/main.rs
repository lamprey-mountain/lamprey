use clap::Parser;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use kerosene::cli::Args;
use kerosene_core::{config::Config, error::ServerResult};
use kerosene_rest::Routes;
use tracing::info;

#[tokio::main]
async fn main() -> ServerResult<()> {
    let args = Args::parse();

    let config: Config = Figment::new()
        .merge(Toml::file(args.config))
        .merge(Env::raw())
        .extract()?;

    kerosene_core::observability::init(&config);

    info!("booting up with config: {:#?}", config);

    // TODO: set up crypto
    // rustls::crypto::ring::default_provider()
    //     .install_default()
    //     .expect("Failed to install rustls crypto provider");

    // TODO: copy crate-backend/src/serve/mod.rs
    let globals = todo!();

    let router = Routes::new_api()
        .into_axum()
        // #[cfg(feature = "embed-frontend")]
        // .layer(frontend) // or .nest? or .fallback?
        // .layer(one)
        // .layer(two)
        // .layer(three)
        .with_state(globals);
    // axum::serve(listener, router)

    // let server = Server::init_from_config(config).await?;
    // server.serve().await?;

    Ok(())
}
