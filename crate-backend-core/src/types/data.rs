use common::v1::types::redex::{RedexFormat, RedexLocation, RedexMetadata};

#[derive(Debug, Clone)]
pub struct DataScriptVersion {
    pub format: RedexFormat,
    pub location: RedexLocation,
    pub metadata: RedexMetadata,
}
