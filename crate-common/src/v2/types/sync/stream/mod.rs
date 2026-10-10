//! Stream definitions for the sync protocol

use lamprey_macros::record;

/// A protocol that can be used with a stream
///
/// Syncing takes place over multiple streams. Each stream can have a different
/// protocol. This defines a protocol that can be used with a stream.
pub trait StreamProtocol {
    /// the initial message sent when opening this stream
    type Initial;

    /// what the client sends
    type Command;

    /// what the server sends
    type Event;

    /// whether commands and events should be sent via datagram if possible
    ///
    /// a standard stream is opened to send initial through and to control the
    /// lifetime of the stream. ie. when that stream is closed, datagrams sent to that
    /// stream should be dropped.
    fn is_datagram() -> bool {
        false
    }

    /// whether messages sent via this stream should be compressed
    fn is_compressed() -> bool {
        true
    }

    // TODO: allow custom serialization/codec formats per stream
    // eg. maybe i want dispatches to use json or msgpack, but for documents to use binary
}

// NOTE: do i include basic stuff (heartbeats, resuming, errors) for every protocol or just sync?
// NOTE: maybe i could add more stream commands as an alternative to using rest? for stuff like typing indicators, etc.
// NOTE: i should probably split auth (hello) and global dispatches instead of reusing the stream

pub mod channel;
pub mod document;
pub mod flume;
pub mod hello;
// pub mod invite;
pub mod member_list;
// pub mod redex;
pub mod room;
// pub mod user;
pub mod voice;

/// stream header byte
///
/// used to identify what type a stream is. sent as a handshake when opening
/// a stream. sent by the side that opens the stream, which is generally the
/// client.
#[record]
#[derive(Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamHeader {
    Hello = 0x00,

    // subscriptions
    MemberList = 0x10,
    Channel = 0x11,
    Room = 0x12,
    User = 0x13,
    Redex = 0x14,
    Invite = 0x15,

    // voice
    Voice = 0x20,

    // documents
    Document = 0x30,

    // flumes
    Flume = 0x40,
}

impl From<StreamHeader> for u8 {
    fn from(header: StreamHeader) -> Self {
        header as u8
    }
}

impl TryFrom<u8> for StreamHeader {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(StreamHeader::Hello),
            0x10 => Ok(StreamHeader::MemberList),
            0x11 => Ok(StreamHeader::Channel),
            0x12 => Ok(StreamHeader::Room),
            0x13 => Ok(StreamHeader::User),
            0x14 => Ok(StreamHeader::Redex),
            0x15 => Ok(StreamHeader::Invite),
            0x20 => Ok(StreamHeader::Voice),
            0x30 => Ok(StreamHeader::Document),
            0x40 => Ok(StreamHeader::Flume),
            _ => Err(value),
        }
    }
}
