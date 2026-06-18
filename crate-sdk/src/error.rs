#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing required builder field: {0}")]
    MissingBuilderField(String),

    #[error("invalid header value")]
    InvalidHeader,

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("serde_json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("tokio_tungstenite error: {0}")]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),

    // TODO: remove
    #[error("anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    // TODO: remove
    #[error("other error: {0}")]
    Other(String),
}
