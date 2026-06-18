use common::v1::types::ApplicationId;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub lamprey_token: String,
    pub lamprey_base_url: Option<String>,
    pub lamprey_ws_url: Option<String>,
    pub lamprey_cdn_url: Option<String>,
    pub lamprey_application_id: ApplicationId,
    pub discord_token: String,
    pub otel_trace_endpoint: Option<String>,
    pub rust_log: String,
}
