use lamprey_macros::record;
use url::Url;

use crate::{
    v1::types::{
        MediaId, Mime,
        misc::{binary::Binary, hashes::Hashes},
    },
    v2::types::media::MediaMetadata,
};

/// encrypted data for media
#[record]
pub struct EncryptedMedia {
    /// the id of the media
    pub id: MediaId,

    /// media struct for decrypted content
    pub info: EncryptedMediaInfo,

    /// the algorithm used for encryption
    pub params: EncryptedMediaParams,
}

/// parameters used to encrypt a piece of media
#[record]
#[serde(tag = "alg")]
pub enum EncryptedMediaParams {
    /// aes 256-bit in gcm
    #[serde(rename = "A256GCM")]
    Aes256GCM {
        /// the key used for encryption (32 bytes)
        key: Binary<32>,

        /// initialization vector (12 bytes)
        iv: Binary<12>,
    },
}

#[record]
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
    pub hashes: Hashes,
    // /// the thumbnail for this piece of media
    // TODO: add this
    // TODO: enforce that thumnails cannot have thumbnails
    // pub thumbnail: Option<Box<EncryptedMedia>>,
    // gifv?
}

/// the algorithm used to encrypt a piece of media
#[record]
#[derive(PartialEq, Eq, Copy)]
pub enum EncryptedMediaAlgorithm {
    /// aes 256-bit in gcm
    #[serde(rename = "A256GCM")]
    Aes256GCM,
}

impl PartialEq for EncryptedMedia {
    fn eq(&self, other: &Self) -> bool {
        // WARN: should i only check id here?
        // self.id == other.id && self.info == other.info && self.params == other.params
        self.id == other.id
    }
}

impl Eq for EncryptedMedia {}
