use futures::FutureExt;
use futures::Stream;
use futures::future::BoxFuture;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use wtransport::{Connection, RecvStream, SendStream};

use super::Transport;

pub struct WebtransportTransport {
    connection: Connection,
    accept_future: Option<
        BoxFuture<'static, Result<(SendStream, RecvStream), wtransport::error::ConnectionError>>,
    >,
}

pub struct WebtransportStream {
    send: SendStream,
    recv: RecvStream,
}

impl WebtransportStream {
    pub fn new(send: SendStream, recv: RecvStream) -> Self {
        Self { send, recv }
    }
}

impl WebtransportTransport {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            accept_future: None,
        }
    }
}

impl AsyncRead for WebtransportStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for WebtransportStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.send).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.send).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.send).poll_shutdown(cx)
    }
}

impl Stream for WebtransportTransport {
    type Item = WebtransportStream;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.accept_future.is_none() {
            let conn = self.connection.clone();
            self.accept_future = Some(Box::pin(async move { conn.accept_bi().await }));
        }

        if let Some(fut) = &mut self.accept_future {
            match fut.poll_unpin(cx) {
                Poll::Ready(Ok((send, recv))) => {
                    self.accept_future = None;
                    Poll::Ready(Some(WebtransportStream { send, recv }))
                }
                Poll::Ready(Err(_)) => {
                    self.accept_future = None;
                    Poll::Ready(None)
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Pending
        }
    }
}

impl Transport for WebtransportTransport {
    type TransportStream = WebtransportStream;
}
