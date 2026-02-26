use serde::Deserialize;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rust_log: String,
    pub database_url: String,
    pub s3: ConfigS3,
    pub thumb_sizes: Vec<u32>,
    pub otel_trace_endpoint: Option<String>,

    #[serde(default = "default_cache_media")]
    pub cache_media: u64,

    #[serde(default = "default_cache_emoji")]
    pub cache_emoji: u64,

    /// configuration for nats
    ///
    /// if None, use polling for wait query param
    pub nats: Option<ConfigNats>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigS3 {
    pub bucket: String,
    pub endpoint: Url,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigNats {
    /// the address of the nats server
    #[serde(default = "default_nats_addr")]
    pub addr: String,

    /// path to a nats credential file, if authentication is required
    pub credentials: Option<PathBuf>,
}

fn default_nats_addr() -> String {
    "localhost:4222".to_string()
}

fn default_cache_media() -> u64 {
    1000
}

fn default_cache_emoji() -> u64 {
    1_000_000
}
