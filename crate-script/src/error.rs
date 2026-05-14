use common::v1::types::redex::error::RedexError;
use rquickjs::Exception;

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "javascript")]
    #[error("rquickjs: {0}")]
    Rquickjs(#[from] rquickjs::Error),

    #[cfg(feature = "wasm")]
    #[error("wasmtime: {0}")]
    Wasmtime(#[from] wasmtime::Error),

    #[error("validation errors: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("broadcast channel send failed: {0}")]
    BroadcastSend(String),

    #[error("broadcast channel recv failed: {0}")]
    BroadcastRecv(String),

    #[error("watch channel changed failed: {0}")]
    WatchChanged(String),

    #[error("extraction data is None")]
    ExtractionDataMissing,

    #[error("runtime error: {message}")]
    RuntimeError { message: String, stack: String },

    #[error("{0}")]
    Api(RedexError),

    #[error("not yet implemented")]
    Unimplemented,
}

impl Error {
    pub fn from_exception<'js>(exception: Exception<'js>) -> Self {
        Self::RuntimeError {
            message: exception
                .message()
                .unwrap_or_else(|| "Unknown JS error".to_string()),
            stack: exception
                .stack()
                .unwrap_or_else(|| "No stack trace".to_string()),
        }
    }
}
