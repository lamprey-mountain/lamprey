//! api errors

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{application::Scope, redex::error::RedexError, Permission};

mod codes;
mod http_conversions;

pub type ApiResult<T> = Result<T, ApiError>;

pub use codes::ErrorCode;

/// an error that may be returned from the api
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ApiError {
    /// human readable error message
    pub message: String,

    /// error code
    pub code: ErrorCode,

    /// errors in the request body
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub fields: Vec<ErrorField>,

    /// required room permissions
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required_permissions: Vec<Permission>,

    /// required server permissions
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required_permissions_server: Vec<Permission>,

    /// required oauth scopes
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub required_scopes: Vec<Scope>,

    /// unacknowledged warnings
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub warnings: Vec<Warning>,

    /// moderator-set message for automod
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub automod_message: Option<String>,

    /// ratelimit that you ran into
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ratelimit: Option<Ratelimit>,

    /// errors with your script
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub script: Vec<RedexError>,
}

/// warnings that require forcing
///
/// generally, this means you must pass ?force=true in the url. if you like to
/// live life on the edge, you can always pass ?force.
// maybe require header instead? `X-Force: Warning1, Warning2`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Warning {
    /// this role is applied to one or more room member
    RoleNotEmpty,

    /// this tag is applied to one or more post
    TagNotEmpty,
    // this will revoke view access to existing thread members
    // this will revoke view access to existing rsvpers
    // this will remove all permission overwrites and sync access with parent channel
}

/// an error that may be returned from the sync websocket
#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncError {
    /// invalid sequence number (connection may be too old)
    #[error("invalid sequence number (connection may be too old)")]
    InvalidSeq,

    /// you were sent a `Ping` but didn't respond with a `Pong` in time
    #[error("you were sent a `Ping` but didn't respond with a `Pong` in time")]
    Timeout,

    /// you tried to do something that you can't do
    #[error("you tried to do something that you can't do")]
    Unauthorized,

    /// you tried to do something before sending a `Hello` or `Resume`
    #[error("you tried to do something before sending a `Hello` or `Resume`")]
    Unauthenticated,

    /// you tried to send a `Hello` or `Resume` but were already authenticated
    #[error("you tried to send a `Hello` or `Resume` but were already authenticated")]
    AlreadyAuthenticated,

    /// the token sent in `Hello` or `Resume` is invalid
    #[error("the token sent in `Hello` or `Resume` is invalid")]
    AuthFailure,

    /// you sent data that i couldn't decode. make sure you're encoding payloads as utf-8 json as text.
    #[error(
        "you sent data that i couldn't decode. make sure you're encoding payloads as utf-8 json as text."
    )]
    InvalidData,
}

/// a field that has an error
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ErrorField {
    /// path to this field inside the request object
    pub key: Vec<String>,

    /// human readable error message
    // TODO: remove this, generate from `ty`?
    // re-add { message: String } to ErrorFieldType::Other
    pub message: String,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: ErrorFieldType,
}

/// the type of error in the field
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename = "type")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ErrorFieldType {
    /// this field was required but not specified
    Required,

    /// the specified number is out of range
    // NOTE: should these be usize?
    Range { min: Option<u64>, max: Option<u64> },

    /// the specified string or array length is out of range
    // NOTE: should these be usize?
    Length { min: Option<u64>, max: Option<u64> },

    /// the incorrect type was passed
    Type { got: String, expected: String },

    /// some other validation error
    Other,
}

impl ErrorFieldType {
    /// construct a `ErrorFieldType::Length`
    pub fn length(min: u64, max: u64) -> Self {
        Self::Length {
            min: Some(min),
            max: Some(max),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Ratelimit {
    /// how many seconds to wait until retrying
    pub retry_after: f64,

    /// if this is a global ratelimit
    ///
    /// if false, this only affects this bucket
    pub global: bool,
}

impl ApiError {
    #[inline]
    pub fn with_message(code: ErrorCode, message: String) -> Self {
        Self {
            message: message,
            ..Self::from_code(code)
        }
    }

    #[inline]
    pub fn from_code(code: ErrorCode) -> Self {
        Self {
            message: code.to_string(),
            code,
            fields: vec![],
            required_permissions: vec![],
            required_permissions_server: vec![],
            required_scopes: vec![],
            warnings: vec![],
            automod_message: None,
            ratelimit: None,
            script: vec![],
        }
    }

    /// prefix all fields with a path for nested validation
    pub fn nested(self, path: &[String]) -> Self {
        Self {
            fields: self
                .fields
                .into_iter()
                .map(|mut err| {
                    let mut key = path.to_vec();
                    key.extend(err.key);
                    err.key = key;
                    err
                })
                .collect(),
            ..self
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

impl SyncError {
    /// get the websocket close code for this error
    pub fn code(&self) -> u16 {
        match self {
            SyncError::InvalidData => 1007,
            SyncError::Unauthorized => 3003,
            SyncError::Unauthenticated => 3000,
            SyncError::Timeout => 3008,
            SyncError::AuthFailure => 4004,
            SyncError::AlreadyAuthenticated => 4005,
            SyncError::InvalidSeq => 4007,
        }
    }
}
