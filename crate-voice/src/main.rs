use anyhow::Result;
use figment::providers::{Env, Format, Toml};
use lamprey_backend_core::{config::Config, observability};
use lamprey_voice::Sfu;
use tracing::debug;

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Toml::file("sfu.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    observability::init_service(&config, "lamprey-voice")?;

    debug!("hello, world!");

    // let _config_voice = config
    //     .voice
    //     .as_ref()
    //     .expect("missing voice field in config; cannot start sfu");

    // // TODO: validate network interfaces before starting
    // if let Err(e) = util::select_host_address_ipv4(config_voice.host_ipv4.as_deref()) {
    //     error!(
    //         "IPv4 configuration error: {}. A usable IPv4 interface is required.",
    //         e
    //     );
    //     process::exit(1);
    // }

    // if let Err(e) = util::select_host_address_ipv6(config_voice.host_ipv6.as_deref()) {
    //     error!(
    //         "IPv6 configuration error: {}. A usable IPv6 interface is required.",
    //         e
    //     );
    //     process::exit(1);
    // }

    let _handle = Sfu::serve(config).await;

    // TODO: proper shutdown handling
    futures::future::pending::<()>().await;

    Ok(())
}
