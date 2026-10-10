use futures::Stream;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait Transport: Stream<Item = Self::TransportStream> + Send + 'static {
    type TransportStream: TransportStream;
}

pub trait TransportStream: AsyncWrite + AsyncRead + Send + Unpin + 'static {}

// Automatically implement TransportStream for any type that meets the bounds
impl<T> TransportStream for T where T: AsyncWrite + AsyncRead + Send + Unpin + 'static {}

#[cfg(feature = "webtransport")]
pub mod webtransport;
