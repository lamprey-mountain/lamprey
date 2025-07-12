use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// The address to bind the SFU http server to
    pub host: String,

    /// The url of the backend api
    pub api_url: String,

    /// The token to authenticate with the backend
    pub token: String,

    #[serde(default = "default_rust_log")]
    pub rust_log: String,
}

fn default_rust_log() -> String {
    "info,voice=trace".to_string()
}
