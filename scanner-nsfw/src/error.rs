use axum::{http::StatusCode, response::IntoResponse};
use scanner_nsfw::ScannerError;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("file not found")]
    NotFound,

    #[error("scan error: {0}")]
    Scan(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<ScannerError> for ScanError {
    fn from(err: ScannerError) -> Self {
        match err {
            ScannerError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => ScanError::NotFound,
            ScannerError::Io(e) => ScanError::Scan(e.to_string()),
            ScannerError::Image(e) => ScanError::Scan(e.to_string()),
            ScannerError::Tensor(e) => ScanError::Internal(e.to_string()),
            ScannerError::Internal(e) => ScanError::Internal(e),
        }
    }
}

impl IntoResponse for ScanError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match self {
            ScanError::NotFound => (StatusCode::NOT_FOUND, "File not found".to_owned()),
            ScanError::Scan(e) => (StatusCode::BAD_REQUEST, format!("Scan error: {e}")),
            ScanError::Internal(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {e}"),
            ),
        };
        (status, body).into_response()
    }
}
