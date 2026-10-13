use lamprey_macros::record;
use thiserror::Error;

#[record]
#[derive(Error)]
pub enum ErrorCode {
    /// application did not respond in time
    #[error("application did not respond in time")]
    Timeout,
}
