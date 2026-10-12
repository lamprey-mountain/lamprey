use std::marker::PhantomData;

use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use lamprey::v2::types::sync::{
    stream::StreamProtocol,
    transport::{Encoding, webtransport::Params},
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec};

use crate::prelude::*;
use crate::v2::transport::TransportStream;

pub struct Codec<P> {
    params: Params,
    _phantom: PhantomData<P>,
}

pub struct CodecClient<P> {
    params: Params,
    framer: LengthDelimitedCodec,
    _phantom: PhantomData<P>,
}

pub struct CodecServer<P> {
    params: Params,
    framer: LengthDelimitedCodec,
    _phantom: PhantomData<P>,
}

impl<P> Codec<P>
where
    P: StreamProtocol,
    P::Initial: Serialize + DeserializeOwned,
{
    pub fn new(params: Params) -> Self {
        Self {
            params,
            _phantom: PhantomData,
        }
    }

    /// server: accept a new stream
    pub async fn accept<S: TransportStream>(
        self,
        stream: S,
    ) -> Result<(P::Initial, Framed<S, CodecServer<P>>)> {
        // read initial payload
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
        let initial_bytes = framed
            .next()
            .await
            .ok_or_else(|| Error::BadStatic("Stream closed before Initial payload"))?
            .map_err(|e| Error::Internal(e.to_string()))?;
        let initial: P::Initial = match self.params.encoding {
            Encoding::Json => serde_json::from_slice(&initial_bytes)?,
            Encoding::Msgpack => rmp_serde::from_slice(&initial_bytes)?,
        };

        // switch codecs
        let framed = framed.map_codec(|framer| CodecServer {
            params: self.params,
            framer,
            _phantom: PhantomData,
        });

        Ok((initial, framed))
    }

    /// client: connect to a stream
    pub async fn connect<S: TransportStream>(
        self,
        stream: S,
        initial: P::Initial,
    ) -> Result<Framed<S, CodecClient<P>>> {
        // send initial payload
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
        let initial_bytes = match self.params.encoding {
            Encoding::Json => serde_json::to_vec(&initial)?,
            Encoding::Msgpack => rmp_serde::to_vec_named(&initial)?,
        };
        framed
            .send(Bytes::from(initial_bytes))
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;

        // switch codecs
        let framed = framed.map_codec(|framer| CodecClient {
            params: self.params,
            framer,
            _phantom: PhantomData,
        });

        Ok(framed)
    }
}

impl<P: StreamProtocol> CodecClient<P> {
    pub fn encoding(&self) -> Encoding {
        self.params.encoding
    }
}

impl<P: StreamProtocol> CodecServer<P> {
    pub fn encoding(&self) -> Encoding {
        self.params.encoding
    }
}

impl<P: StreamProtocol> Decoder for CodecServer<P>
where
    P::Command: DeserializeOwned,
{
    type Item = P::Command;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        match self
            .framer
            .decode(src)
            .map_err(|e| Error::Internal(e.to_string()))?
        {
            Some(bytes) => {
                let msg = match self.encoding() {
                    Encoding::Json => serde_json::from_slice(&bytes)?,
                    Encoding::Msgpack => rmp_serde::from_slice(&bytes)?,
                };
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}

impl<P: StreamProtocol> Encoder<P::Event> for CodecServer<P>
where
    P::Event: Serialize,
{
    type Error = Error;

    fn encode(&mut self, item: P::Event, dst: &mut BytesMut) -> Result<()> {
        let bytes = match self.encoding() {
            Encoding::Json => serde_json::to_vec(&item)?,
            Encoding::Msgpack => rmp_serde::to_vec_named(&item)?,
        };

        self.framer
            .encode(Bytes::from(bytes), dst)
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(())
    }
}

impl<P: StreamProtocol> Encoder<P::Command> for CodecClient<P>
where
    P::Command: Serialize,
{
    type Error = Error;

    fn encode(&mut self, item: P::Command, dst: &mut BytesMut) -> Result<()> {
        let bytes = match self.encoding() {
            Encoding::Json => serde_json::to_vec(&item)?,
            Encoding::Msgpack => rmp_serde::to_vec_named(&item)?,
        };

        self.framer
            .encode(Bytes::from(bytes), dst)
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(())
    }
}

impl<P: StreamProtocol> Decoder for CodecClient<P>
where
    P::Event: DeserializeOwned,
{
    type Item = P::Event;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        match self
            .framer
            .decode(src)
            .map_err(|e| Error::Internal(e.to_string()))?
        {
            Some(bytes) => {
                let msg = match self.encoding() {
                    Encoding::Json => serde_json::from_slice(&bytes)?,
                    Encoding::Msgpack => rmp_serde::from_slice(&bytes)?,
                };
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}

// ========================

// use std::marker::PhantomData;
// use std::pin::Pin;

// use async_compression::tokio::io::{ZlibDecoder, ZlibEncoder};
// use bytes::{BufMut, Bytes, BytesMut};
// use lamprey::v1::types::SyncFormat;
// use serde::{Serialize, de::DeserializeOwned};
// use tokio::io::{AsyncRead, AsyncWrite};
// use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec};

// use crate::prelude::*;

// /// A codec for serializing and deserializing Sync V2 payloads.
// pub struct SyncCodec<C, E> {
//     format: SyncFormat,
//     inner: LengthDelimitedCodec,
//     _marker: PhantomData<(C, E)>,
// }

// impl<C, E> SyncCodec<C, E> {
//     pub fn new(format: SyncFormat) -> Self {
//         Self {
//             format,
//             inner: LengthDelimitedCodec::new(),
//             _marker: PhantomData,
//         }
//     }
// }

// impl<C, E> Decoder for SyncCodec<C, E>
// where
//     C: DeserializeOwned,
// {
//     type Item = C;
//     type Error = Error;

//     fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
//         match self.inner.decode(src) {
//             Ok(Some(bytes)) => {
//                 let msg = match self.format {
//                     SyncFormat::Json => serde_json::from_slice(&bytes)?,
//                     SyncFormat::Msgpack => rmp_serde::from_slice(&bytes)?,
//                 };
//                 Ok(Some(msg))
//             }
//             Ok(None) => Ok(None),
//             Err(e) => Err(Error::Internal(format!("decode error: {e}"))),
//         }
//     }
// }

// impl<C, E> Encoder<E> for SyncCodec<C, E>
// where
//     E: Serialize,
// {
//     type Error = Error;

//     fn encode(&mut self, item: E, dst: &mut BytesMut) -> Result<()> {
//         let bytes = match self.format {
//             SyncFormat::Json => serde_json::to_vec(&item)?,
//             SyncFormat::Msgpack => rmp_serde::to_vec_named(&item)?,
//         };

//         self.inner
//             .encode(Bytes::from(bytes), dst)
//             .map_err(|e| Error::Internal(format!("encode error: {e}")))?;
//         Ok(())
//     }
// }

// // A helper trait to allow boxing AsyncRead + AsyncWrite
// pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Send + Unpin + 'static {}
// impl<T: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncReadWrite for T {}

// /// Wraps a raw TransportStream with optional zlib compression and framing.
// pub fn wrap_stream<S, C, E>(
//     stream: S,
//     format: SyncFormat,
//     compress: bool,
// ) -> Framed<Box<dyn AsyncReadWrite>, SyncCodec<C, E>>
// where
//     S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
//     C: DeserializeOwned + Send + 'static,
//     E: Serialize + Send + 'static,
// {
//     let io: Box<dyn AsyncReadWrite> = if compress {
//         // We compose the decoder on top of the encoder
//         // Wait, ZlibDecoder takes an AsyncRead, ZlibEncoder takes an AsyncWrite.
//         // async_compression's tokio wrappers usually wrap either read or write.
//         // To wrap a bidirectional stream, we can use `tokio::io::split` on a custom struct,
//         // but `async_compression` provides `ZlibEncoder` for writers and `ZlibDecoder` for readers.
//         // For simplicity in a bidirectional wrapper, we can combine them.
//         Box::new(CompressedIo::new(stream))
//     } else {
//         Box::new(stream)
//     };

//     Framed::new(io, SyncCodec::new(format))
// }

// // Helper to wrap a bidirectional stream with both ZlibEncoder (for writes) and ZlibDecoder (for reads)
// struct CompressedIo<S> {
//     // We actually need to split the stream, or use read/write half if supported.
//     // For simplicity, we can use `tokio::io::split` or `tokio::io::duplex`.
//     // But we can also just implement AsyncRead by delegating to ZlibDecoder
//     // and AsyncWrite by delegating to ZlibEncoder.
//     // To do that, we'd need shared access to the inner stream.
//     // The easiest way is to use tokio::io::split if `S` is AsyncRead + AsyncWrite.
//     reader: ZlibDecoder<tokio::io::ReadHalf<S>>,
//     writer: ZlibEncoder<tokio::io::WriteHalf<S>>,
// }

// impl<S> CompressedIo<S>
// where
//     S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
// {
//     fn new(stream: S) -> Self {
//         let (r, w) = tokio::io::split(stream);
//         Self {
//             reader: ZlibDecoder::new(r),
//             writer: ZlibEncoder::new(w),
//         }
//     }
// }

// impl<S> AsyncRead for CompressedIo<S>
// where
//     S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
// {
//     fn poll_read(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//         buf: &mut tokio::io::ReadBuf<'_>,
//     ) -> std::task::Poll<std::io::Result<()>> {
//         Pin::new(&mut self.reader).poll_read(cx, buf)
//     }
// }

// impl<S> AsyncWrite for CompressedIo<S>
// where
//     S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
// {
//     fn poll_write(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//         buf: &[u8],
//     ) -> std::task::Poll<std::io::Result<usize>> {
//         Pin::new(&mut self.writer).poll_write(cx, buf)
//     }

//     fn poll_flush(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<std::io::Result<()>> {
//         Pin::new(&mut self.writer).poll_flush(cx)
//     }

//     fn poll_shutdown(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<std::io::Result<()>> {
//         Pin::new(&mut self.writer).poll_shutdown(cx)
//     }
// }
