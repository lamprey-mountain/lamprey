use serde::{Deserialize, Serialize};

/// configuration for the `ly` command
#[derive(Debug, Deserialize, Serialize)]
pub struct LyConfig {
    pub logins: Vec<Login>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Login {
    pub name: String,
    pub api_url: String,
    pub token: String,
    pub default: bool,
}
