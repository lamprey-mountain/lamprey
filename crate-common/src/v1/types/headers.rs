//! headers used in lamprey

use http::HeaderName;

pub mod content_rating;

pub use content_rating::{ContentRatingDisposition, ContentRatingType, ContentRatings};

// TODO: rename `x-` to `lamprey-`

/// federation signing header: the hostname of the server thats sending this request
pub const HEADER_ORIGIN: HeaderName = HeaderName::from_static("x-origin");

/// federation signing header: the timestamp of this request
pub const HEADER_TIMESTAMP: HeaderName = HeaderName::from_static("x-timestamp");

/// federation signing header: the signature of this request
pub const HEADER_SIGNATURE: HeaderName = HeaderName::from_static("x-signature");

/// federation signing header: the public key that was used to sign this request
pub const HEADER_PUBKEY: HeaderName = HeaderName::from_static("x-pubkey");

/// non-standard header defining the rating of some content
///
/// `content-rating` will have multiple space separated `ContentRating`s
pub const HEADER_CONTENT_RATING: HeaderName = HeaderName::from_static("content-rating");
