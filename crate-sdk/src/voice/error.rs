use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceError {
    /// a symphonia error
    #[error("{0}")]
    Symphonia(#[from] symphonia::core::errors::Error),

    /// a std io error
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("network error: {0}")]
    Net(#[from] str0m::error::NetError),

    #[error("no matching track")]
    NoMatchingTrack,
    // #[error("rtc peer is dead")]
    // Dead,

    // #[error("other unknown error")]
    // Other,
}
