use http::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName};
use tower_http::cors::CorsLayer;

const UPLOAD_OFFSET: HeaderName = HeaderName::from_static("upload-offset");
const UPLOAD_LENGTH: HeaderName = HeaderName::from_static("upload-length");
const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");
const REASON: HeaderName = HeaderName::from_static("x-reason");
const PUPPET_ID: HeaderName = HeaderName::from_static("x-puppet-id");

pub fn cors() -> CorsLayer {
    CorsLayer::very_permissive()
        .expose_headers([CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            UPLOAD_OFFSET,
            UPLOAD_LENGTH,
            IDEMPOTENCY_KEY,
            REASON,
            PUPPET_ID,
        ])
}
