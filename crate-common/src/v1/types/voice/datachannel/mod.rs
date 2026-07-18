//! datachannel messages

use std::marker::PhantomData;

use bytes::BytesMut;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "voice_application")]
pub mod application;

pub mod position;

#[cfg(feature = "voice_sendfile")]
pub mod sendfile;

pub mod speaking;

pub use position::{PositionDatagram, PositionDatagramUpdate};
pub use speaking::{SpeakingDatagram, SpeakingFlags};
// TODO:
// pub use application::...;
// pub use sendfile::...;

use thiserror::Error;

/// the initial byte that is sent
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum ProtocolType {
    /// speaking status
    Speaking = 0x01,

    /// speaker position
    Position = 0x02,

    /// application: broadcast data to all peers
    #[cfg(feature = "voice_application")]
    ApplicationBroadcast = 0x10,

    /// application: create a channel to connect directly to one peer
    #[cfg(feature = "voice_application")]
    ApplicationConnect = 0x11,

    /// application: a channel was created
    #[cfg(feature = "voice_application")]
    ApplicationConnected = 0x12,

    /// sendfile: control channel
    #[cfg(feature = "voice_sendfile")]
    Sendfile = 0x20,

    /// sendfile: sending a file
    #[cfg(feature = "voice_sendfile")]
    SendfileSend = 0x21,

    /// sendfile: downloading/receiving a file
    #[cfg(feature = "voice_sendfile")]
    SendfileRecv = 0x22,
}

impl ProtocolType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(ProtocolType::Speaking),
            0x02 => Some(ProtocolType::Position),
            #[cfg(feature = "voice_application")]
            0x10 => Some(ProtocolType::ApplicationBroadcast),
            #[cfg(feature = "voice_application")]
            0x11 => Some(ProtocolType::ApplicationConnect),
            #[cfg(feature = "voice_application")]
            0x12 => Some(ProtocolType::ApplicationConnected),
            #[cfg(feature = "voice_sendfile")]
            0x20 => Some(ProtocolType::Sendfile),
            #[cfg(feature = "voice_sendfile")]
            0x21 => Some(ProtocolType::SendfileSend),
            #[cfg(feature = "voice_sendfile")]
            0x22 => Some(ProtocolType::SendfileRecv),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, Clone, Error)]
pub enum DatagramDecodeError {
    /// unexpected end of stream
    #[error("unexpected end of stream")]
    UnexpectedEof,

    /// invalid data
    #[error("invalid data")]
    InvalidData,
}

// TODO(?): maybe just use serde + msgpack?
// though for some stuff like Speaking/Position, i really need to keep messages compact
pub trait Datagram: Sized {
    /// encode this into an output buffer
    fn encode<B: bytes::BufMut>(&self, buf: &mut B) -> usize;

    /// decode this from a buffer
    fn decode<B: bytes::Buf>(buf: &mut B) -> Result<Self, DatagramDecodeError>;
}

/// a supported protocol for a datachannel
pub trait Protocol {
    /// the initial datagram sent by the opener
    type Header: Datagram;

    /// the datagrams sent by the client
    type Command: Datagram;

    /// the datagrams sent by the sfu
    type Event: Datagram;

    // /// which side creates the datachannel
    // const OPENER: ProtocolOpener;

    // /// the protocol byte to use
    // const BYTE: ProtocolByte;
}

pub enum ProtocolOpener {
    Client,
    Server,
}

/// an empty datagram, in case a protocol doesn't have a header/command/event.
pub struct EmptyDatagram;

impl Datagram for EmptyDatagram {
    fn encode<B: bytes::BufMut>(&self, _buf: &mut B) -> usize {
        0
    }

    fn decode<B: bytes::Buf>(_buf: &mut B) -> Result<Self, DatagramDecodeError> {
        Ok(EmptyDatagram)
    }
}

/// utility to decode a stream of datagrams
pub struct FramedDecoder<D> {
    buf: BytesMut,
    _phantom: PhantomData<D>,
}

impl<D: Datagram> FramedDecoder<D> {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::new(),
            _phantom: PhantomData,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<D>, DatagramDecodeError> {
        self.buf.extend_from_slice(bytes);

        let mut results = Vec::new();
        loop {
            if self.buf.is_empty() {
                break;
            }

            match D::decode(&mut self.buf) {
                Ok(item) => results.push(item),
                Err(DatagramDecodeError::UnexpectedEof) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(results)
    }
}
