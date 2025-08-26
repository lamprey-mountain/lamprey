use axum::response::{IntoResponse, Response};
use http::StatusCode;

#[derive(Debug)]
#[allow(unused)] // TEMP
pub enum Error {
    NotFound,
    BadRequest,
    Database(sqlx::Error),
    ImageError(image::ImageError),
}

pub type Result<T> = core::result::Result<T, Error>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND.into_response(),
            Error::BadRequest => StatusCode::BAD_REQUEST.into_response(),
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Error::ImageError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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
