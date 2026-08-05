//! Media scanner types for automated media scanning.
//!
//! These types define the request and response formats for external media scanning
//! services (e.g., NSFW detection, malware scanning).

use lamprey_macros::record;

/// A request to scan a media file.
///
/// Sent to external media scanning services configured via [`ConfigMediaScanner`](crate::config::ConfigMediaScanner).
#[record]
pub struct ScanRequest {
    /// The path to the media file to scan.
    pub path: String,
}

/// The response from a media scanning service.
#[record]
pub struct MediaScanResponse {
    /// The confidence score of the scan, from 0.0 to 1.0.
    pub score: f64,

    /// An optional message providing additional context about the scan result.
    pub message: Option<String>,
}
