use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Error)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum VoiceErrorCode {
    /// unknown track
    #[error("unknown mid")]
    UnknownMid,

    /// unknown rid
    #[error("unknown rid")]
    UnknownRid,
}
