use std::sync::Arc;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde::Serialize;
use tracing::error;

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("bad request")]
    BadRequest,

    #[error("database error: {0}")]
    Database(Arc<sqlx::Error>),

    #[error("image error: {0}")]
    ImageError(Arc<image::ImageError>),

    #[error("opendal error: {0}")]
    Opendal(Arc<opendal::Error>),

    #[error("invalid range")]
    BadRange,

    #[error("ffmpeg error")]
    Ffmpeg,

    #[error("tempfile error: {0}")]
    Tempfile(Arc<std::io::Error>),

    #[error("async tempfile error: {0}")]
    AsyncTempfile(Arc<async_tempfile::Error>),
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ErrorCode {
    NotFound,
    BadRequest,
    Database,
    ImageError,
    Opendal,
    BadRange,
    Ffmpeg,
    Tempfile,
    AsyncTempfile,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorJson {
    code: ErrorCode,
    message: String,
}

pub type Result<T> = core::result::Result<T, Error>;

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadRequest => StatusCode::BAD_REQUEST,
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ImageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Opendal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::BadRange => StatusCode::RANGE_NOT_SATISFIABLE,
            Error::Ffmpeg => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Tempfile(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::AsyncTempfile(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> ErrorCode {
        match self {
            Error::NotFound => ErrorCode::NotFound,
            Error::BadRequest => ErrorCode::BadRequest,
            Error::Database(_) => ErrorCode::Database,
            Error::ImageError(_) => ErrorCode::ImageError,
            Error::Opendal(_) => ErrorCode::Opendal,
            Error::BadRange => ErrorCode::BadRange,
            Error::Ffmpeg => ErrorCode::Ffmpeg,
            Error::Tempfile(_) => ErrorCode::Tempfile,
            Error::AsyncTempfile(_) => ErrorCode::AsyncTempfile,
        }
    }

    fn to_json(&self) -> ErrorJson {
        ErrorJson {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        error!("responding with error: {self}");
        (self.status_code(), Json(self.to_json())).into_response()
    }
}

impl From<async_tempfile::Error> for Error {
    fn from(value: async_tempfile::Error) -> Self {
        Error::AsyncTempfile(Arc::new(value))
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Tempfile(Arc::new(value))
    }
}

impl From<opendal::Error> for Error {
    fn from(err: opendal::Error) -> Self {
        if err.kind() == opendal::ErrorKind::NotFound {
            Error::NotFound
        } else {
            Error::Opendal(Arc::new(err))
        }
    }
}

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Error::ImageError(Arc::new(value))
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Database(Arc::new(err)),
        }
    }
}

impl From<Arc<Error>> for Error {
    fn from(value: Arc<Error>) -> Self {
        value.as_ref().clone()
    }
}
