use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    v1::types::{misc::binary::Binary, MediaId, Mime},
    v2::types::media::{HashType, MediaMetadata},
};

/// encrypted data for media
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EncryptedMedia {
    /// the id of the media
    pub id: MediaId,

    /// media struct for decrypted content
    pub media: EncryptedMediaInfo,

    /// the algorithm used for encryption
    pub alg: MediaEncryptionAlg,

    /// the key used for encryption
    // TODO: verify length is correct
    pub key: Binary<256>,

    /// initialization vector
    // TODO: verify length is correct
    pub iv: Binary<256>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EncryptedMediaInfo {
    pub filename: String,
    pub alt: Option<String>,
    pub size: u64,
    pub content_type: Mime,
    pub source_url: Option<Url>,
    pub metadata: MediaMetadata,
    // pub scans: Vec<MediaScan>,
    // pub ratings: ContentRatings,
    /// hashes of decrypted content
    pub hashes: HashMap<HashType, String>,
    // /// the thumbnail for this piece of media
    // TODO: add this
    // TODO: enforce that thumnails cannot have thumbnails
    // pub thumbnail: Option<Box<EncryptedMedia>>,
    // gifv?
}

/// the algorithm used to encrypt a piece of media
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaEncryptionAlg {
    /// aes 256-bit in gcm
    #[cfg_attr(feature = "serde", serde(rename = "A256GCM"))]
    Aes256GCM,
}
