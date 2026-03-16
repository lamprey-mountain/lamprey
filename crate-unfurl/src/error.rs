use tokio::task::JoinError;

#[derive(thiserror::Error, Debug)]
pub enum UnfurlError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTML parsing failed: {0}")]
    Parse(String),

    #[error("Unsupported protocol")]
    UnsupportedProtocol,

    #[error("No plugin could handle the response")]
    NoPluginMatch,

    #[error("Forbidden from unfurling this url")]
    Forbidden,

    #[error(transparent)]
    JoinError(#[from] JoinError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
