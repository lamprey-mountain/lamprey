use axum::{Json, response::IntoResponse};
use http::StatusCode;

pub use lamprey::v1::types::error::{ApiError, ApiResult, ErrorCode};

/// any internal server error
// TODO: add more variants (if needed?)
// the current error type is extremely large, i might not need every variant but some of them do seem important
#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    /// an internal error has occured
    #[error("Internal error: {0}")]
    Internal(Box<dyn std::error::Error>),

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
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Api(e) => e.code.status(),
            Self::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

// TODO: deprecate and remove
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

impl From<opentelemetry_otlp::ExporterBuildError> for ServerError {
    fn from(value: opentelemetry_otlp::ExporterBuildError) -> Self {
        ServerError::Internal(Box::new(value))
    }
}

impl From<tracing::subscriber::SetGlobalDefaultError> for ServerError {
    fn from(value: tracing::subscriber::SetGlobalDefaultError) -> Self {
        ServerError::Internal(Box::new(value))
    }
}

impl From<tracing_subscriber::filter::ParseError> for ServerError {
    fn from(value: tracing_subscriber::filter::ParseError) -> Self {
        ServerError::Internal(Box::new(value))
    }
}

pub trait LegacyErrorExt<T> {
    /// cast all errors into `ServerError::Internal`
    fn cast_internal(self) -> Result<T, ServerError>;
}

impl<T> LegacyErrorExt<T> for lamprey_backend_core::Result<T> {
    fn cast_internal(self) -> Result<T, ServerError> {
        self.map_err(|err| ServerError::Internal(Box::new(err)))
    }
}
