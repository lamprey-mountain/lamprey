use bytes::Bytes;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    v1::types::{Mime, UserId, misc::hashes::Hashes},
    v2::types::media::MediaMetadata,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendfileId(pub u32);

#[derive(Debug, Clone)]
pub enum SendfileEvent {
    /// new sendfile instance created
    ///
    /// multiple events are also sent on initial connection for the client to populate their sendfile list.
    Created(SendfileCreated),

    /// a sendfile instance was destroyed
    Destroyed(SendfileId),
}

#[derive(Debug, Clone)]
pub struct SendfileSendHeader {
    pub metadata: SendfileMetadata,
}

#[derive(Debug, Clone)]
pub enum SendfileSendCommand {
    /// upload a chunk of data for a sendfile instance
    ///
    /// the sfu tries to forward this to as many requesting peers as possible
    Upload(Bytes),

    /// destroy this sendfile instance
    Destroy {
        /// whether to wait for file transfers to complete
        wait: bool,
    },
}

#[derive(Debug, Clone)]
pub enum SendfileSendEvent {
    /// sfu needs a set of chunks to forward to a peer
    ///
    /// the sfu should request parts of a file instead of all at once to prevent too much backpressure
    Needs { selectors: Vec<SendfileSelector> },
    // /// an error occured
    // Error(SendfileError),
}

#[derive(Debug, Clone)]
pub enum SendfileRecvCommand {
    /// request chunks of this file
    ///
    /// sending another `Request` will *replace* the request. make sure to wait for `Done` first.
    ///
    /// the client should generally request parts of a file instead of all at once to prevent too much backpressure
    Request(SendfileSelector),

    /// ask the sfu to stop/pause sending any data
    ///
    /// can be resumed with another `Request` dispatch
    Stop,
}

#[derive(Debug, Clone)]
pub enum SendfileRecvEvent {
    /// a chunk of requested data
    Chunk { chunk_id: u32, data: Bytes },

    /// sfu finished forwarding all requested data
    ///
    /// if the user is missing any data, send another `Request`
    Done(SendfileId),

    /// this sendfile instance was destroyed
    ///
    /// the file can no longer be downloaded
    Destroyed,
}

/// a sendfile instance was created
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendfileCreated {
    pub id: SendfileId,
    pub user_id: UserId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SendfileCreate,

    /// the total number of chunks
    pub chunk_count: u32,

    /// the size of each chunk
    ///
    /// generally should be 16 KiB to 64 KiB
    pub chunk_size: u32,
}

/// metadata for a sendfile instance
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendfileMetadata {
    pub filename: String,
    pub size: u64,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Hashes::is_empty")
    )]
    pub hashes: Hashes,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub alt: Option<String>,

    /// what the creator says the mime type is; untrusted and should not be relied on
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub content_type: Option<Mime>,

    /// what the creator says the media metadata is; untrusted and should not be relied on
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub metadata: Option<MediaMetadata>,
}

/// a selector for a set of chunks
#[derive(Debug, Clone)]
pub enum SendfileSelector {
    /// a range of chunks
    Range {
        /// the chunk to start at, inclusive
        start: u32,

        /// the chunk to end at, exclusive
        end: u32,
    },

    /// a bitfield of chunks
    Bitfield {
        /// the chunk to start at, inclusive
        start: u32,

        /// the bits
        bitfield: u64,
    },
}

impl SendfileSelector {
    /// select a single chunk
    pub fn single(chunk: u32) -> Self {
        Self::Range {
            start: chunk,
            end: chunk + 1,
        }
    }

    pub fn range(start: u32, end: u32) -> Self {
        todo!()
    }

    pub fn bitfield(start: u32, bitfield: u64) -> Self {
        todo!()
    }
}

/// an error occured with the sendfile system
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum SendfileError {
    /// unknown sendfile id
    UnknownSendfile = 0,

    /// invalid sendfile chunk specifier
    ///
    /// ie. chunks that don't exist were sent
    InvalidChunks = 1,

    /// the sfu rejected the upload
    ///
    /// chunk_id or data length was invalid
    InvalidUpload = 2,

    /// invalid dispatch command
    InvalidCommand = 3,
}

// TODO: impl Datagram, Protocol
pub struct SendfileProtocol;
pub struct SendfileRecvProtocol;
pub struct SendfileSendProtocol;

impl Protocol for SendfileProtocol {}
impl Protocol for SendfileRecvProtocol {}
impl Protocol for SendfileSendProtocol {}
