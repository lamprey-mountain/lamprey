use futures::Stream;
use tokio::io::{AsyncRead, AsyncWrite};

/// The underlying physical connection (like a WebTransport session) used for a
/// logical sync connection.
///
/// Transports don't handle bytes on their own, but yield a [`TransportStream`]
/// for each newly opened stream. Streams are (currently always) started
/// by the client. When the transport closes, it will stop yielding
/// [`TransportStream`]s.
pub trait Transport: Stream<Item = Self::TransportStream> + Send + 'static {
    /// The type of physical stream yielded by this transport.
    type TransportStream: TransportStream;
}

/// The underlying physical byte stream for a logical sync stream.
///
/// These streams are bidirectional, ordered, and reliable.
pub trait TransportStream: AsyncWrite + AsyncRead + Send + Unpin + 'static {}

// NOTE: do i want to blanket impl this?
impl<T> TransportStream for T where T: AsyncWrite + AsyncRead + Send + Unpin + 'static {}

// TODO: pub mod websocket;

#[cfg(feature = "webtransport")]
pub mod webtransport;
