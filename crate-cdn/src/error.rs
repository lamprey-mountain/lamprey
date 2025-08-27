use axum::response::{IntoResponse, Response};
use http::StatusCode;

#[derive(Debug, thiserror::Error)]
#[allow(unused)] // TEMP
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("bad request")]
    BadRequest,

    #[error("database error: {0}")]
    Database(sqlx::Error),

    #[error("image error: {0}")]
    ImageError(image::ImageError),

    #[error("invalid range")]
    BadRange,

    #[error("not modified")]
    NotModified,
}

pub type Result<T> = core::result::Result<T, Error>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND.into_response(),
            Error::BadRequest => StatusCode::BAD_REQUEST.into_response(),
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Error::ImageError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Error::BadRange => StatusCode::RANGE_NOT_SATISFIABLE.into_response(),
            Error::NotModified => StatusCode::NOT_MODIFIED.into_response(),
        }
    }
}

impl From<opendal::Error> for Error {
    fn from(_value: opendal::Error) -> Self {
        Error::NotFound
    }
}

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Error::ImageError(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Database(err),
        }
    }
}
