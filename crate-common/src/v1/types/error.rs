//! api errors

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// an error that may be returned from the api
#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Error {
    #[error("user is suspended")]
    UserSuspended,
}

/// an error that may be returned from the sync error
#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ErrorSync {
    #[error("connection is too old")]
    TooOld,

    /// an api error
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
