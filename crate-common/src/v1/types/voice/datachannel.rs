//! datachannel messages

use bitflags::bitflags;
use bytes::Bytes;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::{
    v1::types::{Mime, UserId, misc::hashes::Hashes, voice::Mid},
    v2::types::media::MediaMetadata,
};

/// protocol for a datachannel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum DatachannelProtocol {
    /// speaking data
    Speaking,

    /// speaker position
    // TODO: support this
    Position,

    /// file transfer
    // TODO: support this
    Sendfile,

    /// arbitrary application data broadcast to all peers
    // TODO: support this
    Application,
    // /// controlling the connection
    // Control,
}

/// datachannel data
// TODO: rename to DatachannelDatagram
#[derive(Debug, Clone)]
pub enum Datachannel {
    Speaking(SpeakingDatagram),
    Position(PositionDatagram),
    Sendfile(SendfileDatagram),
    Application(ApplicationDatagram),
    // Control(ControlDatagram),
}

/// speaking datagram
///
/// ## binary
///
/// - 16 byte mid
/// - 16 byte user id
/// - 1 byte flags
#[derive(Debug, Clone, PartialEq, Eq)]
// TODO: use binary for serde
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpeakingDatagram {
    pub mid: Mid,
    pub user_id: UserId,
    pub flags: SpeakingFlags,
}

// TODO: merge into SpeakingDatagram
/// a message sent from the peer to indicate that they're speaking (among other things)
///
/// ## binary
///
/// - 16 byte mid
/// - 1 byte flags
// could be fun to add other filters? like lowpass, reverb, etc (can be done client side)
#[derive(Debug, Clone)]
// TODO: use binary for serde
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Speaking {
    pub mid: Mid,
    pub flags: SpeakingFlags,
}

// TODO: merge into SpeakingDatagram
/// a message sent to the client to indicate that someone is speaking
///
/// ## binary
///
/// - 16 byte mid
/// - 16 byte user id
/// - 1 byte flags
#[derive(Debug, Clone)]
// TODO: use binary for serde
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpeakingWithUserId {
    pub mid: Mid,
    pub user_id: UserId,
    pub flags: SpeakingFlags,
}

/// a speaking datachannel message from a sfu
///
/// ## binary
///
/// - 16 byte mid
/// - 16 user id
/// - 1 byte flags
#[derive(Debug, Clone)]
pub struct SpeakingDatagramResponse {
    pub mid: Mid,
    pub user_id: UserId,
    pub flags: SpeakingFlags,
}

/// an audio position datagram
///
/// ## binary
///
/// - 4 byte mid
/// - 4 byte user id
/// - 16 byte update (repeated until the end of the datagram)
#[derive(Debug, Clone)]
pub struct PositionDatagram {
    pub mid: Mid,
    pub user_id: UserId,
    pub updates: Vec<PositionDatagramUpdate>,
}

#[derive(Debug, Clone)]
pub enum PositionDatagramUpdate {
    /// set the position of an audio source
    Position { x: f32, y: f32, z: f32 },

    /// set where an audio source is pointing
    Direction { x: f32, y: f32, z: f32 },
}

/// a datagram for a file being sent
#[derive(Debug, Clone)]
pub struct SendfileDatagram {
    pub dispatche: SendfileDispatch,
}

/// either a command or event
///
/// ## binary
///
/// - defers to inner command/event; u8 tag shouldn't have any overlap
#[derive(Debug, Clone)]
pub enum SendfileDispatch {
    Command(SendfileCommand),
    Event(SendfileEvent),
}

/// unique identifier for a sendfile instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendfileId {
    /// the user who this sendfile belongs to
    pub user_id: UserId,

    /// incrementing numeric id
    pub id: u32,
}

// peer -> sfu
#[derive(Debug, Clone)]
pub enum SendfileCommand {
    /// create a new file send instance
    ///
    /// ## binary
    ///
    /// - u8 tag = `0`
    /// - u32 id
    /// - serialized SendfileCreate
    Create(u32, SendfileCreate),

    /// request part or all of a file from a user
    ///
    /// sending a `Request` for a `SendfileId` that you haven't gotten a `Done` for will *replace* the request
    ///
    /// the peer should request parts of a file instead of all at once to prevent too much backpressure
    ///
    /// ## binary
    ///
    /// - u8 tag = `1`
    Request(SendfileId, SendfileChunks),

    /// pause a request for a file
    ///
    /// can be resumed with another `Request` dispatch
    ///
    /// ## binary
    ///
    /// - u8 tag = `2`
    Pause(SendfileId),

    /// cancel a request for a file
    ///
    /// ## binary
    ///
    /// - u8 tag = `3`
    Abort(SendfileId),

    /// a chunk of data for a sendfile instance
    ///
    /// the sfu tries to broadcast `Download` to as many requesting peers as possible
    ///
    /// ## binary
    ///
    /// - u8 tag = `4`
    /// - u32 chunk id
    Upload { id: SendfileId, data: Bytes },

    /// destroy a sendfile instance
    ///
    /// the `bool` says whether it should wait for file transfers to complete.
    /// if `false`, immediately destroy the sendfile instance.
    ///
    /// ## binary
    ///
    /// - u8 tag = `5`
    /// - u32 id
    /// - u8 bool
    Destroy(u32, bool),
}

// peer <- sfu
#[derive(Debug, Clone)]
pub enum SendfileEvent {
    /// new sendfile instance created
    ///
    /// multiple events are also sent on initial connection for the client to populate their sendfile list.
    ///
    /// ## binary
    ///
    /// - u8 tag = `127`
    /// - binary SendfileCreated
    Created(SendfileCreated),

    /// sfu finished forwarding all requested data
    ///
    /// if the user is missing any data, send another `Request`
    ///
    /// ## binary
    ///
    /// - u8 tag = `128`
    /// - binary SendfileId
    Done(SendfileId),

    /// sfu needs a set of chunks to forward to a peer
    ///
    /// the sfu should request parts of a file instead of all at once to prevent too much backpressure
    ///
    /// ## binary
    ///
    /// - u8 tag = `129`
    /// - u32 chunk id
    /// - binary SendfileChunks
    Needs { id: u32, chunks: SendfileChunks },

    /// a chunk of data for a sendfile instance
    ///
    /// ## binary
    ///
    /// - u8 tag = `130`
    /// - binary SendfileId
    /// - u32 chunk id
    /// - u32 data length
    /// - length data bytes
    Download {
        id: SendfileId,
        chunk_id: u32,
        data: Bytes,
    },

    /// a sendfile instance was destroyed
    ///
    /// ## binary
    ///
    /// - u8 tag = `131`
    /// - binary SendfileId
    Destroyed(SendfileId),

    /// an error occured
    ///
    /// ## binary
    ///
    /// - u8 tag = `132`
    /// - binary SendfileError
    Error(SendfileError),
}

/// an error occured with the sendfile system
///
/// ## binary
///
/// - u8 code
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

/// create a new sendfile instance
///
/// ## binary
///
/// - u32 length
/// - length serialized json data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendfileCreate {
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

/// a sendfile instance was created
///
/// ## binary
///
/// - u32 length
/// - length serialized json data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendfileCreated {
    pub id: SendfileId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SendfileCreate,

    /// the total number of chunks
    pub chunk_count: u32,

    /// the size of each chunk
    ///
    /// generally should be 16 KiB to 64 KiB
    pub chunk_size: u32,
}

/// a set of chunks for sendfile
///
/// ## binary
///
/// - u32 length of chunks
/// - length chunks
#[derive(Debug, Clone)]
pub struct SendfileChunks {
    pub chunks: Vec<SendfileChunksType>,
}

/// an individual chunk selector
///
/// ## binary
///
/// - u8 enum tag
/// - chunk data
#[derive(Debug, Clone)]
pub enum SendfileChunksType {
    /// a range of chunks
    ///
    /// ## binary
    ///
    /// - u32 start
    /// - u32 end
    Range {
        /// the chunk to start at, inclusive
        start: u32,

        /// the chunk to end at, exclusive
        end: u32,
    },

    /// a bitfield of chunks
    ///
    /// ## binary
    ///
    /// - u32 start
    /// - u64 bitfield
    Bitfield {
        /// the chunk to start at, inclusive
        start: u32,

        /// the bits
        bitfield: u64,
    },
}

/// a datagram for arbitrary application data
// TODO: have some way to manage application channels. maybe have some kind of control channel?
#[derive(Debug, Clone)]
pub struct ApplicationDatagram {
    pub data: Vec<u8>,
}

/// a datagram for controlling the connection
// NOTE: unsure how necessary this is
#[derive(Debug, Clone)]
pub struct ControlDatagram {
    pub dispatch: ControlDispatch,
}

// TODO: implement
// NOTE: probably just use serde + json
#[derive(Debug, Clone)]
pub enum ControlDispatch {
    // // NOTE: probably not necessary
    // /// send a signalling message via webrtc
    // Signalling(SignallingCommand),

    // TODO: managing datachannels?
}

bitflags! {
    /// Flags for speaking
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct SpeakingFlags: u8 {
        /// whether to send audio
        const AUDIO = 1 << 0;

        /// whether a speaking indicator should be sent
        const INDICATOR = 1 << 1;

        /// whether to use priority speaker
        const PRIORITY = 1 << 2;

        /// whether to broadcast to multiple channels
        const BROADCAST = 1 << 3;
    }
}

impl Speaking {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16 + 1);
        bytes.extend_from_slice(&self.mid.0);
        bytes.push(self.flags.bits());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        if bytes.len() != 17 {
            return Err(());
        }
        let mut mid = [0u8; 16];
        mid.copy_from_slice(&bytes[0..16]);
        let flags = SpeakingFlags::from_bits_truncate(bytes[16]);
        Ok(Speaking {
            mid: Mid(mid),
            flags,
        })
    }
}

impl SpeakingWithUserId {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16 + 1 + 16);
        bytes.extend_from_slice(&self.mid.0);
        bytes.extend_from_slice(self.user_id.as_bytes());
        bytes.push(self.flags.bits());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        if bytes.len() != 33 {
            return Err(());
        }
        let mut mid = [0u8; 16];
        mid.copy_from_slice(&bytes[0..16]);
        let mut peer_bytes = [0u8; 16];
        peer_bytes.copy_from_slice(&bytes[16..32]);
        Ok(SpeakingWithUserId {
            mid: Mid(mid),
            flags: SpeakingFlags::from_bits_truncate(bytes[32]),
            user_id: UserId::from(Uuid::from_bytes(peer_bytes)),
        })
    }
}

// TODO: serde for all of these

impl SpeakingDatagram {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}

impl PositionDatagram {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}

impl SendfileDatagram {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}

impl ApplicationDatagram {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}

impl Datachannel {
    /// serialize this datachannel payload into bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    /// get the datachannel protocol used for this message
    pub fn protocol(&self) -> DatachannelProtocol {
        todo!()
    }

    /// parse this datachannel payload from protocol and bytes
    pub fn from_bytes(_protocol: DatachannelProtocol, _bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}
