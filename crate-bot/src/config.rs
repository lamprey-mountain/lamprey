use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub rust_log: String,
    pub token: String,
    pub base_url: Option<String>,
    pub ws_url: Option<String>,
}
