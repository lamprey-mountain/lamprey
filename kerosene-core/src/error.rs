use std::sync::Arc;

use axum::{Json, response::IntoResponse};
use http::StatusCode;

pub use lamprey::v1::types::error::{ApiError, ApiResult, ErrorCode};

/// an opaque internal server error
///
/// this type is cheap to clone and erases the underlying error type. use it for
/// errors that only need to be logged, not inspected or matched on by callers.
#[derive(Clone, Debug)]
pub struct InternalError(Arc<dyn std::error::Error + Send + Sync + 'static>);

#[derive(Debug)]
struct MessageError(String);

/// any internal server error
// TODO: add more variants (if needed?)
// the current error type is extremely large, i might not need every variant but some of them do seem important
#[derive(thiserror::Error, Debug, Clone)]
pub enum ServerError {
    /// an internal error has occured
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),

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

pub type ServerResult<T> = ::core::result::Result<T, ServerError>;
pub use ::core::result::Result as CoreResult;
pub type Result<T, E = ServerError> = ::core::result::Result<T, E>;

impl InternalError {
    /// wrap an error
    #[inline]
    pub fn new<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self(Arc::new(error))
    }

    /// wrap an already-erased error
    #[inline]
    pub fn from_boxed(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self(Arc::from(error))
    }

    /// create an internal error from a message
    #[inline]
    pub fn message(message: impl Into<String>) -> Self {
        Self::new(MessageError(message.into()))
    }

    /// attempt to downcast the underlying error
    #[inline]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: std::error::Error + 'static,
    {
        self.0.downcast_ref()
    }

    /// access the underlying error
    #[inline]
    pub fn as_dyn_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.0
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for InternalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for InternalError {
    fn from(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::from_boxed(error)
    }
}

impl From<Arc<dyn std::error::Error + Send + Sync + 'static>> for InternalError {
    fn from(error: Arc<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self(error)
    }
}

impl From<String> for InternalError {
    fn from(message: String) -> Self {
        Self::message(message)
    }
}

impl From<&str> for InternalError {
    fn from(message: &str) -> Self {
        Self::message(message)
    }
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MessageError {}

impl ServerError {
    /// wrap any error into an internal error
    #[inline]
    pub fn internal<E: Into<Box<dyn std::error::Error + Send + Sync>>>(err: E) -> Self {
        Self::Internal(InternalError::from_boxed(err.into()))
    }

    /// create an internal error from a message
    #[inline]
    pub fn internal_message(message: impl Into<String>) -> Self {
        Self::Internal(InternalError::message(message))
    }

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
            ServerError::Internal(err) => {
                ApiError::with_message(ErrorCode::Internal, err.to_string())
            }
            ServerError::Api(err) => *err,
            ServerError::Unimplemented => ApiError::from_code(ErrorCode::Unimplemented),
            ServerError::Unavailable => ApiError::from_code(ErrorCode::Unavailable),
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
        ServerError::internal(value)
    }
}

impl From<tracing::subscriber::SetGlobalDefaultError> for ServerError {
    fn from(value: tracing::subscriber::SetGlobalDefaultError) -> Self {
        ServerError::internal(value)
    }
}

impl From<tracing_subscriber::filter::ParseError> for ServerError {
    fn from(value: tracing_subscriber::filter::ParseError) -> Self {
        ServerError::internal(value)
    }
}
