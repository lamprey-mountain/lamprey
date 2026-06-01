use str0m::error::SdpError;

/// errors that can be emitted from the sfu
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// no voice state exists for this user
    #[error("no voice state exists for this user")]
    NotConnected,

    /// the `Have` message is only sent by the server
    #[error("the `Have` message is only sent by the server")]
    HaveServerOnly,

    #[error("{0}")]
    Rustls(#[from] rustls::Error),

    #[error("websocket error: {0}")]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("invalid auth token: {0}")]
    InvalidAuthToken(String),

    #[error("channel error: {0}")]
    Channel(String),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("sdp error: {0}")]
    Sdp(#[from] SdpError),

    #[error("rtc error: {0}")]
    Rtc(#[from] str0m::RtcError),
}

pub type Result<T> = ::core::result::Result<T, Error>;
