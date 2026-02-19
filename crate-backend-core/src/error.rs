use std::num::{ParseFloatError, ParseIntError};

use axum::extract::multipart::{MultipartError, MultipartRejection};
use axum::{extract::ws::Message, http::StatusCode, response::IntoResponse, Json};
use common::v1::types::application::Scopes;
use common::v1::types::error::{ApiError, SyncError};
use common::v1::types::{MessageEnvelope, MessagePayload, MessageSync};
use opentelemetry_otlp::ExporterBuildError;
use serde_json::json;
use tracing::error;

#[derive(thiserror::Error, Debug)]
// TODO: avoid returning actual error messages to prevent leaking stuff
pub enum Error {
    #[error("sqlx error: {0}")]
    Sqlx(sqlx::Error),
    #[error("blocked by other user")]
    Blocked,
    #[error("missing authentication (not provided or invalid/expired session)")]
    MissingAuth,
    #[error("conflict")]
    Conflict,
    #[error("bad header")]
    BadHeader,
    #[error("session not yet authenticated")]
    UnauthSession,
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    MissingPermissions,
    #[error("bad request: {0}")]
    BadStatic(&'static str),
    #[error("bad request: {0}")]
    BadRequest(String),
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
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("axum error")]
    Axum(#[from] axum::Error),
    #[error("sushi send error: {0}")]
    SushiSend(#[from] tokio::sync::broadcast::error::SendError<MessageSync>),
    #[error("parse int error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("parse float error: {0}")]
    ParseFloat(#[from] ParseFloatError),
    #[error("opendal error: {0}")]
    Opendal(#[from] opendal::Error),
    #[error("migrate error: {0}")]
    SqlxMigrate(#[from] sqlx::migrate::MigrateError),
    #[error("tracing subscriber error: {0}")]
    TracingSubscriber(#[from] tracing::subscriber::SetGlobalDefaultError),
    #[error("log format parse error: {0}")]
    LogFormatParse(#[from] tracing_subscriber::filter::ParseError),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("figment error: {0}")]
    Figment(#[from] figment::Error),
    #[error("url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("compress error: {0}")]
    Compress(#[from] flate2::CompressError),

    #[error("decompress error: {0}")]
    Decompress(#[from] flate2::DecompressError),

    #[error("unmodified")]
    // HACK: not really an error, but still kind of helpful to have here
    NotModified,

    #[error("ffmpeg or ffprobe didn't like seem to like that very much")]
    Ffmpeg,

    #[error("media type error: {0}")]
    Media(#[from] mediatype::MediaTypeError),

    #[error("not yet implemented...")]
    Unimplemented,

    #[error("image error: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("unknown image format")]
    UnknownImageFormat,

    #[error("url embed io error: {0}")]
    UrlEmbed(std::io::Error),

    #[error("url embed error: {0}")]
    UrlEmbedOther(String),

    #[error("validation error: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("lettre error: {0}")]
    Lettre(#[from] lettre::error::Error),

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("email address already exists for this user")]
    EmailAlreadyExists,

    #[error("generic error: {0}")]
    GenericError(String),

    #[error("OtelExporterBuildError: {0}")]
    OtelExporterBuildError(#[from] ExporterBuildError),

    #[error("{0}")]
    ApiError(ApiError),

    #[error("{0}")]
    SyncError(SyncError),

    #[error("{0}")]
    MultipartError(#[from] MultipartError),

    #[error("{0}")]
    MultipartRejection(#[from] MultipartRejection),

    #[error("missing scopes {0:?}")]
    MissingScopes(Scopes),

    #[error("yrs error: {0}")]
    Yrs(#[from] yrs::error::Error),

    #[error("yrs update error: {0}")]
    YrsUpdate(#[from] yrs::error::UpdateError),

    #[error("yrs read error: {0}")]
    YrsRead(#[from] yrs::encoding::read::Error),

    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("tantivy query error: {0}")]
    TantivyQuery(String),

    #[error("nats error: {0}")]
    Nats(#[from] async_nats::Error),

    #[error("nats jetstream error: {0}")]
    NatsJetstream(String),
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Error::NotFound,
            err => Error::Sqlx(err),
        }
    }
}

impl From<axum::http::header::ToStrError> for Error {
    fn from(_value: axum::http::header::ToStrError) -> Self {
        Error::BadHeader
    }
}

impl Error {
    fn get_status(&self) -> StatusCode {
        match self {
            Error::Blocked => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadHeader => StatusCode::BAD_REQUEST,
            Error::Conflict => StatusCode::CONFLICT,
            Error::BadStatic(_) => StatusCode::BAD_REQUEST,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Serde(_) => StatusCode::BAD_REQUEST,
            Error::MissingAuth => StatusCode::UNAUTHORIZED,
            Error::UnauthSession => StatusCode::UNAUTHORIZED,
            Error::TooBig => StatusCode::PAYLOAD_TOO_LARGE,
            Error::MissingPermissions => StatusCode::FORBIDDEN,
            Error::CantOverwrite => StatusCode::CONFLICT,
            Error::ParseInt(_) => StatusCode::BAD_REQUEST,
            Error::ParseFloat(_) => StatusCode::BAD_REQUEST,
            Error::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Error::NotModified => StatusCode::NOT_MODIFIED,
            Error::Validation(_) => StatusCode::BAD_REQUEST,
            Error::ApiError(err) => StatusCode::from_u16(err.code.status()).unwrap(),
            Error::SyncError(err) => match err {
                SyncError::InvalidSeq => StatusCode::BAD_REQUEST,
                SyncError::Timeout => StatusCode::BAD_REQUEST,
                SyncError::Unauthorized => StatusCode::FORBIDDEN,
                SyncError::Unauthenticated => StatusCode::UNAUTHORIZED,
                SyncError::AlreadyAuthenticated => StatusCode::BAD_REQUEST,
                SyncError::AuthFailure => StatusCode::UNAUTHORIZED,
                SyncError::InvalidData => StatusCode::BAD_REQUEST,
            },
            Error::MultipartError(_) => StatusCode::BAD_REQUEST,
            Error::MissingScopes(_) => StatusCode::FORBIDDEN,
            Error::TantivyQuery(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn fake_clone(&self) -> Error {
        match self {
            Error::Blocked => Error::Blocked,
            Error::MissingAuth => Error::MissingAuth,
            Error::BadHeader => Error::BadHeader,
            Error::Conflict => Error::Conflict,
            Error::UnauthSession => Error::UnauthSession,
            Error::NotFound => Error::NotFound,
            Error::MissingPermissions => Error::MissingPermissions,
            Error::ApiError(err) => Error::ApiError(err.clone()),
            Error::SyncError(err) => Error::SyncError(err.clone()),
            Error::BadStatic(s) => Error::BadStatic(s),
            Error::BadRequest(s) => Error::BadRequest(s.clone()),
            Error::TooBig => Error::TooBig,
            Error::Internal(s) => Error::Internal(s.clone()),
            Error::CantOverwrite => Error::CantOverwrite,
            Error::ParseInt(parse_int_error) => Error::ParseInt(parse_int_error.clone()),
            Error::ParseFloat(parse_float_error) => Error::ParseFloat(parse_float_error.clone()),
            Error::Figment(error) => Error::Figment(error.clone()),
            Error::UrlParseError(parse_error) => Error::UrlParseError(*parse_error),
            Error::NotModified => Error::NotModified,
            Error::Ffmpeg => Error::Ffmpeg,
            Error::Media(media_type_error) => Error::Media(*media_type_error),
            Error::Unimplemented => Error::Unimplemented,
            Error::UnknownImageFormat => Error::UnknownImageFormat,
            Error::UrlEmbedOther(s) => Error::UrlEmbedOther(s.to_string()),
            Error::Validation(validation_errors) => Error::Validation(validation_errors.clone()),
            Error::MissingScopes(s) => Error::MissingScopes(s.clone()),
            Error::Tantivy(t) => Error::Tantivy(t.clone()),
            Error::TantivyQuery(s) => Error::TantivyQuery(s.clone()),
            _ => Error::GenericError(self.to_string()),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if let Error::NotModified = self {
            return self.get_status().into_response();
        };

        if let Error::ApiError(e) = self {
            error!("Response api error: {:?}", e);
            (StatusCode::from_u16(e.code.status()).unwrap(), Json(e)).into_response()
        } else {
            error!(
                "Response error: status {}, message {:?}",
                self.get_status(),
                self
            );
            (
                self.get_status(),
                Json(json!({ "error": self.to_string() })),
            )
                .into_response()
        }
    }
}

impl From<Error> for Message {
    fn from(val: Error) -> Self {
        Message::text(
            serde_json::to_string(&MessageEnvelope {
                payload: MessagePayload::Error {
                    error: val.to_string(),
                },
            })
            .expect("error should always be able to be serialized"),
        )
    }
}

impl From<ApiError> for Error {
    fn from(err: ApiError) -> Self {
        Self::ApiError(err)
    }
}

impl From<SyncError> for Error {
    fn from(value: SyncError) -> Self {
        Self::SyncError(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
