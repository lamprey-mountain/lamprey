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

    #[error("sdp error: {0}")]
    Sdp(#[from] str0m::error::SdpError),

    #[error("rtc error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("ice error: {0}")]
    Ice(#[from] str0m::error::IceError),

    #[error("stun error: {0}")]
    Stun(#[from] stunclient::Error),

    #[error("rtc peer is dead")]
    Dead,

    #[error("connection closed")]
    ConnectionClosed,
}
