use std::str::FromStr;

use anyhow::Result;
use figment::providers::{Env, Format, Toml};
use lamprey_backend_core::config::Config;
use lamprey_voice_new::sfu::Sfu;
use tracing::subscriber;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config: Config = figment::Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Toml::file("sfu.toml"))
        .merge(Env::raw())
        .extract()?;

    let sub = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str(&config.rust_log)?)
        .finish();
    subscriber::set_global_default(sub)?;

    let _config_voice = config
        .voice
        .as_ref()
        .expect("missing voice field in config; cannot start sfu");

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

    let _ = Sfu::serve(config).await;

    Ok(())
}
