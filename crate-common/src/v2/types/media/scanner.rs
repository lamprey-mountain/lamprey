//! Media scanner types for automated media scanning.
//!
//! These types define the request and response formats for external media scanning
//! services (e.g., NSFW detection, malware scanning).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// A request to scan a media file.
///
/// Sent to external media scanning services configured via [`ConfigMediaScanner`](crate::config::ConfigMediaScanner).
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ScanRequest {
    /// The path to the media file to scan.
    pub path: String,
}

/// The response from a media scanning service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaScanResponse {
    /// The confidence score of the scan, from 0.0 to 1.0.
    pub score: f64,

    /// An optional message providing additional context about the scan result.
    pub message: Option<String>,
}
