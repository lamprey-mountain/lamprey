use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// The url of the backend api
    pub api_url: String,

    /// The token to authenticate with the backend
    pub token: String,

    #[serde(default = "default_rust_log")]
    pub rust_log: String,

    pub host_ipv4: Option<String>,
    pub host_ipv6: Option<String>,

    /// The number of worker threads to spawn. Defaults to the number of CPU cores.
    pub workers: Option<u8>,

    /// The UDP port to use for all media traffic.
    pub udp_port: u16,
}

fn default_rust_log() -> String {
    "info,voice=trace".to_string()
}
