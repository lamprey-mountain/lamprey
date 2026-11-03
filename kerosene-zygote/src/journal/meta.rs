//! Events stored in the meta journal ([`JournalId::META`]).
//!
//! Each event is one block in the meta journal. Events are encoded as compact
//! binary with the layout `version, tag, ...`.
//!
//! Deleting the meta journal truncates its history. Backends must lazily
//! recreate it the next time an event is appended.

use bytes::{BufMut, Bytes, BytesMut};

use super::JournalId;

const VERSION: u8 = 1;

const TAG_CREATE: u8 = 0;
const TAG_FREEZE: u8 = 1;
const TAG_DELETE: u8 = 2;

/// A journal catalog event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalEvent {
    /// A journal was created.
    Create(JournalId),

    /// A journal was frozen.
    Freeze(JournalId),

    /// A journal was deleted.
    Delete(JournalId),
}

impl JournalEvent {
    /// Encode this event into a compact binary block.
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(16);
        buf.put_u8(VERSION);
        match self {
            Self::Create(id) => {
                buf.put_u8(TAG_CREATE);
                buf.put_u64(id.0);
            }
            Self::Freeze(id) => {
                buf.put_u8(TAG_FREEZE);
                buf.put_u64(id.0);
            }
            Self::Delete(id) => {
                buf.put_u8(TAG_DELETE);
                buf.put_u64(id.0);
            }
        }
        buf.freeze()
    }

    /// Decode an event from a meta journal block.
    pub fn decode(mut data: &[u8]) -> Result<Self, MetaError> {
        let version = take_u8(&mut data)?;
        if version != VERSION {
            return Err(MetaError::Version(version));
        }

        let tag = take_u8(&mut data)?;
        let event = match tag {
            TAG_CREATE => Self::Create(JournalId(take_u64(&mut data)?)),
            TAG_FREEZE => Self::Freeze(JournalId(take_u64(&mut data)?)),
            TAG_DELETE => Self::Delete(JournalId(take_u64(&mut data)?)),
            other => return Err(MetaError::UnknownTag(other)),
        };

        if !data.is_empty() {
            return Err(MetaError::TrailingBytes);
        }
        Ok(event)
    }
}

fn take_u8(data: &mut &[u8]) -> Result<u8, MetaError> {
    let (&byte, rest) = data.split_first().ok_or(MetaError::Truncated)?;
    *data = rest;
    Ok(byte)
}

fn take_bytes<'a>(data: &mut &'a [u8], len: usize) -> Result<&'a [u8], MetaError> {
    if data.len() < len {
        return Err(MetaError::Truncated);
    }
    let (head, rest) = data.split_at(len);
    *data = rest;
    Ok(head)
}

fn take_u64(data: &mut &[u8]) -> Result<u64, MetaError> {
    let bytes = take_bytes(data, 8)?;
    let mut buf = [0; 8];
    buf.copy_from_slice(bytes);
    Ok(u64::from_be_bytes(buf))
}

/// An error that occurred while decoding a meta journal event.
#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    /// the event ended early
    #[error("meta event is truncated")]
    Truncated,

    /// the event used an unsupported version
    #[error("unsupported meta event version {0}")]
    Version(u8),

    /// the event used an unknown tag
    #[error("unknown meta event tag {0}")]
    UnknownTag(u8),

    /// the event had bytes left after decoding
    #[error("meta event has trailing bytes")]
    TrailingBytes,
}
