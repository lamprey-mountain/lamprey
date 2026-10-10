use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::v2::transport::TransportStream;

use async_compression::tokio::bufread::DeflateDecoder;
use async_compression::tokio::write::DeflateEncoder;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, BufReader, ReadBuf, ReadHalf, WriteHalf};

/// compress a stream with deflate
///
/// compatible with web api `new CompressionStream("deflate-raw")`
pub struct Deflate<S> {
    reader: DeflateDecoder<BufReader<ReadHalf<S>>>,
    writer: DeflateEncoder<WriteHalf<S>>,
}

impl<S: TransportStream + AsyncBufRead> Deflate<S> {
    /// create a new Deflate wrapping this transport stream
    pub fn new(stream: S) -> Self {
        let (read_half, write_half) = tokio::io::split(stream);

        Self {
            reader: DeflateDecoder::new(BufReader::new(read_half)),
            writer: DeflateEncoder::new(write_half),
        }
    }
}

impl<S: TransportStream> AsyncRead for Deflate<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl<S: TransportStream> AsyncWrite for Deflate<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}
