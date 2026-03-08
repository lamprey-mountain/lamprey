use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub token: String,
    pub base_url: Option<String>,
    pub ws_url: Option<String>,
    pub music_path: String,
    pub database_url: String,
    pub rust_log: String,
}
