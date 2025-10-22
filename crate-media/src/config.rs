use serde::Deserialize;
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
}

#[derive(Debug, Deserialize)]
pub struct ConfigS3 {
    pub bucket: String,
    pub endpoint: Url,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

fn default_cache_media() -> u64 {
    1000
}

fn default_cache_emoji() -> u64 {
    1_000_000
}
