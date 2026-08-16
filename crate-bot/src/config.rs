use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub rust_log: String,
    pub database_url: String,

    // auth
    pub token: String,
    pub api_url: Option<Url>,
    pub cdn_url: Option<Url>,
    pub sync_url: Option<Url>,

    // remove?
    pub music_path: String,
    // TODO: support this
    // pub otel_trace_endpoint: Option<String>,
}
