use std::num::ParseIntError;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing Authentication header")]
    MissingAuthHeader,
    #[error("bad header")]
    BadHeader,
    #[error("session not yet authenticated")]
    UnauthSession,
    #[error("not found")]
    NotFound,
    #[error("too big :(")]
    TooBig,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("internal error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("can't overwrite already uploaded data!")]
    CantOverwrite,
    #[error("internal error: {0}")]
    Tempfile(#[from] async_tempfile::Error),
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Error::NotFound,
            err => Error::Internal(err.to_string()),
        }
    }
}

impl From<axum::http::header::ToStrError> for Error {
    fn from(_value: axum::http::header::ToStrError) -> Self {
        Error::BadHeader
    }
}

impl From<ParseIntError> for Error {
    fn from(_value: ParseIntError) -> Self {
        Error::BadHeader
    }
}

impl Error {
    fn get_status(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadHeader => StatusCode::BAD_REQUEST,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::MissingAuthHeader => StatusCode::UNAUTHORIZED,
            Error::UnauthSession => StatusCode::UNAUTHORIZED,
            Error::TooBig => StatusCode::PAYLOAD_TOO_LARGE,
            Error::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::CantOverwrite => StatusCode::CONFLICT,
            Error::Tempfile(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.get_status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}
