use std::path::{Path, PathBuf};

use figment::providers::Format;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// RUST_LOG configuration
    pub rust_log: String,

    /// directory containing the nsfw model(s)
    pub models_path: PathBuf,

    /// model file to load, relative to `models_path`
    #[serde(default = "default_model")]
    pub model: PathBuf,

    /// Listen configuration
    #[serde(default = "default_listen")]
    pub listen: ListenConfig,
}

fn default_model() -> PathBuf {
    PathBuf::from("model.onnx")
}

fn default_listen() -> ListenConfig {
    ListenConfig::Tcp {
        address: "127.0.0.1".parse().unwrap(),
        port: 4100,
    }
}

// TODO: refactor out shared scanner types (into kerosene-core?)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ListenConfig {
    Tcp {
        address: std::net::IpAddr,
        port: u16,
    },
    Unix {
        path: PathBuf,
    },
}

impl Config {
    pub fn load(config_path: &Path) -> Result<Self, figment::Error> {
        figment::Figment::new()
            .merge(figment::providers::Toml::file(config_path))
            .merge(figment::providers::Env::raw().only(&["RUST_LOG"]))
            .merge(figment::providers::Env::prefixed("SCANNER_NSFW_"))
            .extract()
    }
}
