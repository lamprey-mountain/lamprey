//! voice error codes

use lamprey_macros::record;
use thiserror::Error;

use crate::util::registry::export_models;

#[record]
#[derive(Error)]
pub enum VoiceErrorCode {
    /// unknown mid
    ///
    /// there isn't any track that exists with this mid
    #[error("unknown mid")]
    UnknownMid,
}

export_models!(VoiceErrorCode);
