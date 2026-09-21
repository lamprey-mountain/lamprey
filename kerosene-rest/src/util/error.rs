use axum::response::IntoResponse;
use http::StatusCode;

use crate::prelude::*;

/// an error that may occur during request extraction
#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    /// invalid content type header
    #[error("invalid content-type")]
    InvalidContentType,

    /// missing content type header
    #[error("missing content-type")]
    MissingContentType,

    /// a multipart field as no name
    #[error("a multipart field has no name")]
    MultipartNamelessField,

    /// multipart payload already exists
    #[error("multipart duplicate payload")]
    MultipartDuplicatePayload,

    /// multipart media already exists
    #[error("multipart duplicate media")]
    MultipartDuplicateMedia,

    /// multipart field already exists
    #[error("multipart duplicate field")]
    MultipartDuplicateField,

    /// missing body
    #[error("missing body")]
    MissingBody,

    /// body too large
    #[error("body too large")]
    BodyTooLarge,
}

pub enum ExtractorRejection {
    ServerError(ServerError),
    ExtractorError(ExtractorError),
    Response(http::Response<Bytes>),
}

impl From<ExtractorError> for ExtractorRejection {
    fn from(err: ExtractorError) -> Self {
        Self::ExtractorError(err)
    }
}

impl From<ServerError> for ExtractorRejection {
    fn from(err: ServerError) -> Self {
        Self::ServerError(err)
    }
}

impl From<ApiError> for ExtractorRejection {
    fn from(err: ApiError) -> Self {
        Self::ServerError(ServerError::Api(Box::new(err)))
    }
}

impl From<http::Response<Bytes>> for ExtractorRejection {
    fn from(value: http::Response<Bytes>) -> Self {
        Self::Response(value)
    }
}

impl IntoResponse for ExtractorRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            ExtractorRejection::ServerError(err) => err.into_response(),
            ExtractorRejection::Response(res) => res.map(|b| b.into()),
            ExtractorRejection::ExtractorError(err) => {
                let status = match err {
                    ExtractorError::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                    _ => StatusCode::BAD_REQUEST,
                };

                let code = match err {
                    // TODO: better error codes
                    _ => ErrorCode::InvalidData,
                };

                let api_error = ApiError::with_message(ErrorCode::InvalidData, err.to_string());
                (status, axum::Json(api_error)).into_response()
            }
        }
    }
}
