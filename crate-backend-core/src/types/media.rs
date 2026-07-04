use common::v2::types::MediaId;

/// media path calculator
pub struct MediaPaths {
    pub(super) prefix: String,
}

impl MediaPaths {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// get the path for the file itself
    pub fn file(&self, media_id: MediaId) -> String {
        format!("{}/file", self.base(media_id))
    }

    /// get the path for the file's embedded thumbnail
    pub fn poster(&self, media_id: MediaId) -> String {
        format!("{}/poster", self.base(media_id))
    }

    /// get the path for the file's (potentially animated) generated thumbnail of a specific size
    pub fn thumb(&self, media_id: MediaId, size: u32, ext: &str) -> String {
        format!("{}/thumb/{}x{}.{}", self.base(media_id), size, size, ext)
    }

    /// get the path for the file's (never animated) generated thumbnail of a specific size
    pub fn thumb_static(&self, media_id: MediaId, size: u32, ext: &str) -> String {
        format!(
            "{}/thumb/{}x{}_static.{}",
            self.base(media_id),
            size,
            size,
            ext
        )
    }

    /// get the path for the transcoded gifv
    pub fn gifv(&self, media_id: MediaId) -> String {
        format!("{}/gifv", self.base(media_id))
    }

    fn base(&self, media_id: MediaId) -> String {
        format!("{}{}", self.prefix, media_id)
    }

    // TODO: stream
    // TODO: trickplay
}
