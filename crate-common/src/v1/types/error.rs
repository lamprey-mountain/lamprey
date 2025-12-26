//! api errors

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::application::Scopes;

/// an error that may be returned from the api
#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Error {
    #[error("user is suspended")]
    UserSuspended,

    #[error("missing scopes {0:?}")]
    MissingScopes(Scopes),
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

    /// an api error
    // NOTE: may be removed later
    #[error("{0}")]
    Api(#[from] Error),
}

// struct Error2 {
//     /// human readable error message
//     message: String,

//     /// error code
//     code: ApiErrorCode,

//     /// errors in the request body
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     fields: Vec<ApiErrorField>,

//     /// required room permissions
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     required_permissions: Vec<String>,

//     /// required server permissions
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     required_permissions_server: Vec<String>,

//     /// required oauth scopes
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     required_scopes: Vec<String>,
// }
