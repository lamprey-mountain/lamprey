use lamprey_macros::record;
use thiserror::Error;

use crate::util::registry::export_models;

#[record]
#[derive(Error)]
pub enum ErrorCode {
    /// application did not respond in time
    #[error("application did not respond in time")]
    Timeout,
}

export_models!(ErrorCode);
