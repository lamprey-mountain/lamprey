use core::fmt;
use std::{collections::HashMap, str::FromStr};

use bytes::Bytes;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::misc::binary::Binary;

/// a set of hashes
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Hashes(pub HashMap<HashType, HashData>);

/// a single hash
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
// TODO: unsure what the ideal max length should be
pub struct HashData(pub Binary<1024>);

/// the type of hash
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(into = "String", try_from = "String")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum HashType {
    /// SHA-512/256
    ///
    /// generate hashes with `openssl dgst -sha512-256 ./path/to/file`
    Sha512_256,

    /// BLAKE3 hash
    ///
    /// generate hashes with `b3sum ./path/to/file`
    Blake3,

    /// Some unknown or unsupported algorithm
    Other(String),
}

impl Hashes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert(&mut self, key: HashType, value: HashData) -> Option<HashData> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &HashType) -> Option<&HashData> {
        self.0.get(key)
    }

    pub fn remove(&mut self, key: &HashType) -> Option<HashData> {
        self.0.remove(key)
    }
}

impl From<HashMap<HashType, HashData>> for Hashes {
    fn from(map: HashMap<HashType, HashData>) -> Self {
        Self(map)
    }
}

impl From<Hashes> for HashMap<HashType, HashData> {
    fn from(hashes: Hashes) -> Self {
        hashes.0
    }
}

impl HashData {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn into_bytes(self) -> Bytes {
        self.0.0
    }
}

// TODO: require validation that value is correct length?
impl From<Vec<u8>> for HashData {
    fn from(value: Vec<u8>) -> Self {
        Self(Binary(Bytes::from(value)))
    }
}

// TODO: require validation that value is correct length?
impl From<Bytes> for HashData {
    fn from(value: Bytes) -> Self {
        Self(Binary(value))
    }
}

impl fmt::Display for HashType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashType::Sha512_256 => write!(f, "Sha512/256"),
            HashType::Blake3 => write!(f, "Blake3"),
            HashType::Other(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for HashType {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Sha512/256" => HashType::Sha512_256,
            "Blake3" => HashType::Blake3,
            other => HashType::Other(other.to_string()),
        })
    }
}

impl From<HashType> for String {
    fn from(h: HashType) -> Self {
        h.to_string()
    }
}

impl TryFrom<String> for HashType {
    type Error = std::convert::Infallible;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}
