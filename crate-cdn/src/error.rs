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

    #[error("invalid range")]
    BadRange,

    #[error("not modified")]
    NotModified,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ErrorCode {
    NotFound,
    BadRequest,
    Database,
    ImageError,
    BadRange,
    NotModified,
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
            Error::BadRange => StatusCode::RANGE_NOT_SATISFIABLE,
            Error::NotModified => StatusCode::NOT_MODIFIED,
        }
    }

    fn code(&self) -> ErrorCode {
        match self {
            Error::NotFound => ErrorCode::NotFound,
            Error::BadRequest => ErrorCode::BadRequest,
            Error::Database(_) => ErrorCode::Database,
            Error::ImageError(_) => ErrorCode::ImageError,
            Error::BadRange => ErrorCode::BadRange,
            Error::NotModified => ErrorCode::NotModified,
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

impl From<opendal::Error> for Error {
    fn from(_value: opendal::Error) -> Self {
        Error::NotFound
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
