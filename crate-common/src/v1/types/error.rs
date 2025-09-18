//! api errors

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// an error that may be returned from the api
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Error {
    #[error("user is suspended")]
    UserSuspended,
}
