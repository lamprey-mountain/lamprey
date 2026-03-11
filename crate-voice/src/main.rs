use std::process;
use std::str::FromStr;

use anyhow::Result;
use figment::providers::{Env, Format, Toml};
use lamprey_voice::{config::Config, sfu::Sfu, util};
use tracing::{error, subscriber};
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
    subscriber::set_global_default(sub)?;

    // Validate network interfaces before starting
    if let Err(e) = util::select_host_address_ipv4(config.host_ipv4.as_deref()) {
        error!(
            "IPv4 configuration error: {}. A usable IPv4 interface is required.",
            e
        );
        process::exit(1);
    }

    if let Err(e) = util::select_host_address_ipv6(config.host_ipv6.as_deref()) {
        error!(
            "IPv6 configuration error: {}. A usable IPv6 interface is required.",
            e
        );
        process::exit(1);
    }

    let _ = Sfu::run(config.clone()).await;

    Ok(())
}
