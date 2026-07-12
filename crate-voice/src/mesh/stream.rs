//! wire format for streams

use crate::prelude::*;
use common::{
    v1::types::voice::{Mid, Rid},
    v2::types::UserId,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

// TODO: move these types to lamprey-common?
// somewhat inspired by https://www.ietf.org/archive/id/draft-lcurley-moq-lite-04.html

/// header byte, the first thing that is sent when a quic stream is opened
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum Header {
    /// authenticate
    Hello = 0x01,

    /// advertise a track
    // NOTE: unnecessary with voice states?
    Announce = 0x02,

    /// subscribe to a track
    Subscribe = 0x03,

    /// fetch data from a track
    Fetch = 0x04,

    /// measure bitrate and latency
    Probe = 0x05,

    /// disconnect
    Goodbye = 0x06,
}

pub struct Blank;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeId(u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscribe {
    /// unique identifier for this subscription
    pub id: SubscribeId,

    // which track to subscribe to
    pub publisher_id: UserId,
    pub track_id: Mid,
    pub layer_id: Option<Rid>,

    pub config: SubscribeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeConfig {
    /// higher means more priority
    pub priority: u8,

    /// which groups to send first
    pub ordered: SubscribeOrder,

    /// max age of a group (in milliseconds) compared to last sent group
    ///
    /// old group streams are reset upon deadline exceeded
    pub max_latency: u32,
}

/// command sent to a subscription stream
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum SubscribeCommand {
    /// create a new subscription
    ///
    /// followed by a serialized `Subscribe`. the first command MUST be a Create. only the first command may be a Create.
    Create = 0x00,

    /// update this subscription's config
    ///
    /// followed by a serialized `SubscribeConfig`.
    Update = 0x01,

    /// acknowledge some data
    Ack = 0x02,

    /// stop receiving data
    Close = 0x03,

    /// request a keyframe
    Keyframe = 0x04,
}

/// whether to transmit groups in ascending or descending order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
enum SubscribeOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Probe {
    // bitrate
    // rtt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProbeResponse {
    // bitrate
    // rtt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Goodbye {
    code: GoodbyeCode,
    // /// where to reconnect
    // ///
    // /// if this is None, don't reconnect. may be the same url.
    // reconnect_url: Option<Url>,
}

// impl fn can_reconnect()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
enum GoodbyeCode {
    /// move to a different voice channel
    Move,

    /// reconnect to the same channel
    Reconnect,

    /// user was logged out
    Deauthenticated,

    /// user lost permission
    Deauthorized,
}

/// a stream of ordered frames
///
/// sent by the peer in response to a Subscription stream
struct Group {
    /// corresponding subscribe id for this group
    subscribe_id: SubscribeId,

    /// sequence number of this group. monotonic, may have gaps.
    seq: u64,
}

/// a chunk of bytes in a group
struct Frame<'a>(&'a [u8]);

/// a wrapper to parse a stream
pub struct MeshStream<T> {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    _t: PhantomData<T>,
}

pub enum AcceptedStream {
    Subscribe(MeshStream<Subscribe>),
    Probe(MeshStream<Probe>),
    Hello(MeshStream<Hello>),
    Goodbye(MeshStream<Goodbye>),
}

impl<T> MeshStream<T> {
    // pub fn new(recv: ()) -> Self {
    //     todo!()
    // }

    // fn parse(&mut self, data: &[u8]) {}
}

impl MeshStream<Blank> {
    pub fn new(send: quinn::SendStream, recv: quinn::RecvStream) -> Self {
        Self {
            send,
            recv,
            _t: PhantomData,
        }
    }

    pub async fn accept(mut self) -> Result<AcceptedStream> {
        let mut header_buf = [0u8; 1];
        self.recv
            .read_exact(&mut header_buf)
            .await
            .expect("TODO: better error handling");
        // TODO: impl Header::from_byte() -> Option<Self>
        // maybe use serde/postcard directly?
        match header_buf[0] {
            0x01 => Ok(AcceptedStream::Hello(MeshStream {
                send: self.send,
                recv: self.recv,
                _t: PhantomData,
            })),
            0x03 => Ok(AcceptedStream::Subscribe(MeshStream {
                send: self.send,
                recv: self.recv,
                _t: PhantomData,
            })),
            0x05 => Ok(AcceptedStream::Probe(MeshStream {
                send: self.send,
                recv: self.recv,
                _t: PhantomData,
            })),
            0x06 => Ok(AcceptedStream::Goodbye(MeshStream {
                send: self.send,
                recv: self.recv,
                _t: PhantomData,
            })),
            other => Err(Error::Backend(format!(
                "Unsupported stream header: {other}"
            ))),
        }
    }
}

impl MeshStream<Subscribe> {
    // these need `send`
    // pub fn configure(&self, subscription: ()) {
    //     todo!()
    // }
    //
    // pub fn ack(&self) {}
    // pub fn keyframe(&self) {}
    // pub fn close(self) {}

    // how do i type this?
    // pub fn recv(self) {}
}
