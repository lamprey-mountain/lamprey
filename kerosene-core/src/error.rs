use axum::{Json, response::IntoResponse};
use http::StatusCode;

pub use lamprey::v1::types::error::{ApiError, ApiResult, ErrorCode};

/// any internal server error
#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    /// an internal error has occured
    #[error("Internal error: {0}")]
    Internal(Box<str>),

    /// an api error
    #[error("{0}")]
    Api(Box<ApiError>),

    /// feature isn't implemented yet
    #[error("not implemented yet...")]
    Unimplemented,

    /// service unavailable
    #[error("service unavailable")]
    // Unavailable(UnavailableReason), // TODO: add Reason?
    Unavailable,
}

pub type ServerResult<T> = std::result::Result<T, ServerError>;

impl ServerError {
    pub fn http_status(&self) -> StatusCode {
        match self {
            ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Api(e) => e.code.status(),
            ServerError::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            ServerError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl From<ServerError> for ApiError {
    fn from(value: ServerError) -> Self {
        match value {
            ServerError::Internal(_) => todo!(),
            ServerError::Api(err) => *err,
            ServerError::Unimplemented => todo!(),
            ServerError::Unavailable => todo!(),
        }
    }
}

impl From<ApiError> for ServerError {
    fn from(e: ApiError) -> Self {
        ServerError::Api(Box::new(e))
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let e: ApiError = self.into();
        (e.code.status(), Json(e)).into_response()
    }
}
