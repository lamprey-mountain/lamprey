//! errors for sync

use kerosene_core::error::{ApiError, ErrorCode, InternalError, ServerError};
use lamprey::v1::types::error::SyncErrorCode;

/// an error that occurred while syncing
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// the client sent invalid data
    #[error("invalid data: {0}")]
    InvalidData(String),

    /// a sync protocol error that is reported to the client with its code
    #[error("{0}")]
    Sync(#[from] SyncErrorCode),

    /// an internal server error
    #[error("internal error: {0}")]
    Internal(InternalError),

    /// a json error while decoding a message
    #[error("json decode error: {0}")]
    JsonDecode(#[source] serde_json::Error),

    /// a json error while encoding a message
    #[error("json encode error: {0}")]
    JsonEncode(#[source] serde_json::Error),

    /// a msgpack decoding error
    #[error("msgpack decode error: {0}")]
    MsgpackDecode(#[from] rmp_serde::decode::Error),

    /// a msgpack encoding error
    #[error("msgpack encode error: {0}")]
    MsgpackEncode(#[from] rmp_serde::encode::Error),

    /// a decompression error
    #[error("decompress error: {0}")]
    Decompress(#[from] flate2::DecompressError),

    /// a compression error
    #[error("compress error: {0}")]
    Compress(#[from] flate2::CompressError),

    /// a transport io error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// a websocket error
    #[error("websocket error: {0}")]
    Websocket(#[from] axum::Error),
}

impl Error {
    /// create an error for invalid client data
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData(message.into())
    }

    /// wrap an error as an internal server error
    pub fn internal<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::Internal(InternalError::from_boxed(error.into()))
    }

    /// create an internal server error from a message
    pub fn internal_message(message: impl Into<String>) -> Self {
        Self::Internal(InternalError::message(message))
    }
}

impl From<Error> for ServerError {
    fn from(err: Error) -> Self {
        // TODO: preserve error codes
        fn invalid_data(message: impl Into<String>) -> ServerError {
            ApiError::with_message(ErrorCode::InvalidData, message.into()).into()
        }

        match err {
            Error::InvalidData(message) => invalid_data(message),
            Error::Sync(code) => invalid_data(code.to_string()),
            Error::Internal(err) => ServerError::Internal(err),
            Error::JsonDecode(err) => invalid_data(err.to_string()),
            Error::JsonEncode(err) => ServerError::internal(err),
            Error::MsgpackDecode(err) => invalid_data(err.to_string()),
            Error::MsgpackEncode(err) => ServerError::internal(err),
            Error::Decompress(err) => invalid_data(err.to_string()),
            Error::Compress(err) => ServerError::internal(err),
            Error::Io(err) => ServerError::internal(err),
            Error::Websocket(err) => ServerError::internal(err),
        }
    }
}

pub type Result<T, E = Error> = ::core::result::Result<T, E>;
