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
}

pub type Result<T> = ::core::result::Result<T, Error>;
