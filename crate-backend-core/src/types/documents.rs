use bytes::{Buf, BufMut, Bytes, BytesMut};
use common::{
    v1::types::{document::DocumentRevisionId, ids::DocumentBranchId, util::Time},
    v2::types::UserId,
};
use thiserror::Error;
use uuid::Uuid;

// TODO: validate lengths, avoid casting `as u16` as that can lead to silent truncation

/// compact serialized set of document changes
///
/// used for cold storage
pub struct Changeset {
    pub metadata: ChangesetMetadata,

    /// concatenated update bytes, sliced via offset/len in ChangesetItem
    pub updates: Bytes,
}

#[derive(Debug, Clone)]
pub struct ChangesetMetadata {
    pub magic: u32, // b"CSET" or similar
    pub version: u8,
    pub checksum: u32, // crc32 of `updates`
    pub start_time: Time,
    pub start_seq: u64,
    pub authors: Vec<UserId>,      // u16 len prefix, u128 each
    pub items: Vec<ChangesetItem>, // u16 len prefix
}

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ChangesetItem {
    /// user made a change to the document
    Change(ChangesetChange),

    /// nothing happened for a while
    Delay {
        /// time in milliseconds since last change
        time_delta: u64,
    },

    /// another branch was merged into this branch
    Merge {
        /// time in milliseconds since last change
        time_delta: u32,

        /// the exact revision that was merged
        revision_id: DocumentRevisionId,
    },
}

#[derive(Error, Debug)]
pub enum ChangesetError {
    #[error("invalid magic bytes")]
    InvalidMagic,

    #[error("checksum mismatch")]
    ChecksumMismatch,

    #[error("too short")]
    TooShort,

    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("invalid uuid: {0}")]
    InvalidUuid(String),

    #[error("invalid item type")]
    InvalidItemType,
}

#[derive(Debug, Clone)]
pub struct ChangesetChange {
    /// time in milliseconds since the last change
    pub time_delta: u32,

    /// the index of the the author's uuid
    ///
    /// user id accessible via `header.authors[author_index]`
    pub author_index: u16,

    /// number of characters added to the document
    pub stat_added: u32,

    /// number of characters removed from the document
    pub stat_removed: u32,

    pub update_offset: u32,
    pub update_len: u32,
}

impl Changeset {
    pub fn into_bytes(mut self) -> Bytes {
        let mut buf = BytesMut::new();

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.updates);
        let checksum = hasher.finalize();
        self.metadata.checksum = checksum;

        self.metadata.write_to(&mut buf);
        buf.extend_from_slice(&self.updates);
        buf.freeze()
    }

    pub fn from_bytes(mut bytes: Bytes) -> Result<Self, ChangesetError> {
        let metadata = ChangesetMetadata::read_from(&mut bytes)?;
        let updates = bytes;

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&updates);
        let checksum = hasher.finalize();

        if checksum != metadata.checksum {
            return Err(ChangesetError::ChecksumMismatch);
        }

        Ok(Changeset { metadata, updates })
    }
}

impl ChangesetMetadata {
    fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u32(self.magic);
        buf.put_u8(self.version);
        buf.put_u32(self.checksum);
        buf.put_i128(self.start_time.into_inner().unix_timestamp_nanos());
        buf.put_u64(self.start_seq);

        buf.put_u16(self.authors.len() as u16);
        for author in &self.authors {
            buf.extend_from_slice(author.into_inner().as_bytes());
        }

        buf.put_u16(self.items.len() as u16);
        for item in &self.items {
            item.write_to(buf);
        }
    }

    fn read_from(buf: &mut Bytes) -> Result<Self, ChangesetError> {
        if buf.remaining() < 4 {
            return Err(ChangesetError::TooShort);
        }
        let magic = buf.get_u32();
        if &magic.to_be_bytes() != b"CSET" {
            return Err(ChangesetError::InvalidMagic);
        }

        let version = buf.get_u8();
        let checksum = buf.get_u32();

        let start_time = Time::from(
            time::OffsetDateTime::from_unix_timestamp_nanos(buf.get_i128())
                .map_err(|e| ChangesetError::InvalidTimestamp(e.to_string()))?,
        );
        let start_seq = buf.get_u64();

        let num_authors = buf.get_u16() as usize;
        let mut authors = Vec::with_capacity(num_authors);
        for _ in 0..num_authors {
            let author_bytes = buf.copy_to_bytes(16);
            let uuid = Uuid::from_slice(&author_bytes)
                .map_err(|e| ChangesetError::InvalidUuid(e.to_string()))?;
            authors.push(UserId::from(uuid));
        }

        let num_items = buf.get_u16() as usize;
        let mut items = Vec::with_capacity(num_items);
        for _ in 0..num_items {
            items.push(ChangesetItem::read_from(buf)?);
        }

        Ok(ChangesetMetadata {
            magic,
            version,
            checksum,
            start_time,
            start_seq,
            authors,
            items,
        })
    }
}

impl ChangesetItem {
    fn write_to(&self, buf: &mut BytesMut) {
        match self {
            ChangesetItem::Change(c) => {
                buf.put_u8(0);
                c.write_to(buf);
            }
            ChangesetItem::Delay { time_delta } => {
                buf.put_u8(1);
                buf.put_u64(*time_delta);
            }
            ChangesetItem::Merge {
                time_delta,
                revision_id,
            } => {
                buf.put_u8(2);
                buf.put_u32(*time_delta);
                buf.extend_from_slice(revision_id.branch_id.into_inner().as_bytes());
                buf.put_u64(revision_id.seq);
            }
        }
    }

    fn read_from(buf: &mut Bytes) -> Result<Self, ChangesetError> {
        match buf.get_u8() {
            0 => Ok(ChangesetItem::Change(ChangesetChange::read_from(buf)?)),
            1 => Ok(ChangesetItem::Delay {
                time_delta: buf.get_u64(),
            }),
            2 => {
                let time_delta = buf.get_u32();
                let branch_id = DocumentBranchId::from(
                    Uuid::from_slice(&buf.copy_to_bytes(16))
                        .map_err(|e| ChangesetError::InvalidUuid(e.to_string()))?,
                );
                let seq = buf.get_u64();
                Ok(ChangesetItem::Merge {
                    time_delta,
                    revision_id: DocumentRevisionId { branch_id, seq },
                })
            }
            _ => Err(ChangesetError::InvalidItemType),
        }
    }
}

impl ChangesetChange {
    fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u32(self.time_delta);
        buf.put_u16(self.author_index);
        buf.put_u32(self.stat_added);
        buf.put_u32(self.stat_removed);
        buf.put_u32(self.update_offset);
        buf.put_u32(self.update_len);
    }

    fn read_from(buf: &mut Bytes) -> Result<Self, ChangesetError> {
        Ok(ChangesetChange {
            time_delta: buf.get_u32(),
            author_index: buf.get_u16(),
            stat_added: buf.get_u32(),
            stat_removed: buf.get_u32(),
            update_offset: buf.get_u32(),
            update_len: buf.get_u32(),
        })
    }
}

// TODO: add more tests
#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use common::v1::types::util::Time;

    #[test]
    fn test_changeset_checksum_validation() {
        let updates = Bytes::from("some updates");
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&updates);
        let checksum = hasher.finalize();

        let metadata = ChangesetMetadata {
            magic: u32::from_be_bytes(*b"CSET"),
            version: 1,
            checksum,
            start_time: Time::now_utc(),
            start_seq: 1,
            authors: vec![],
            items: vec![],
        };

        let changeset = Changeset { metadata, updates };
        let bytes = changeset.into_bytes();

        let decoded = Changeset::from_bytes(bytes).expect("should decode successfully");
        assert_eq!(decoded.metadata.checksum, checksum);
    }
}
