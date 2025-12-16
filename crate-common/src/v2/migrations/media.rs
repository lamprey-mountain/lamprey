use crate::v1::types::media::{Media as V1Media, MediaWithAdmin as V1MediaWithAdmin};
use crate::v2::types::media::{Media, MediaMetadata, MediaStatus};

impl Into<V1Media> for Media {
    fn into(self) -> V1Media {
        V1Media {
            id: self.id,
            filename: self.filename,
            alt: self.alt,
            source: crate::v1::types::MediaTrack {
                info: match self.metadata {
                    MediaMetadata::Image { width, height } => {
                        crate::v1::types::MediaTrackInfo::Image(crate::v1::types::Image {
                            height,
                            width,
                            language: None,
                        })
                    }
                    MediaMetadata::Video {
                        width,
                        height,
                        duration,
                    } => crate::v1::types::MediaTrackInfo::Mixed(crate::v1::types::Mixed {
                        width: Some(width),
                        height: Some(height),
                        duration: Some(duration),
                        language: None,
                    }),
                    MediaMetadata::Audio { duration } => {
                        crate::v1::types::MediaTrackInfo::Mixed(crate::v1::types::Mixed {
                            height: None,
                            width: None,
                            duration: Some(duration),
                            language: None,
                        })
                    }
                    MediaMetadata::Text => {
                        crate::v1::types::MediaTrackInfo::Text(crate::v1::types::Text {
                            language: None,
                        })
                    }
                    MediaMetadata::File => crate::v1::types::MediaTrackInfo::Other,
                },
                size: self.size,
                mime: self.content_type,
                source: if let Some(source_url) = self.source_url {
                    crate::v1::types::TrackSource::Downloaded { source_url }
                } else {
                    crate::v1::types::TrackSource::Uploaded
                },
            },
        }
    }
}

impl Into<Media> for V1Media {
    fn into(self) -> Media {
        let s = self.source;
        Media {
            id: self.id,
            // WARNING: the database is going to need to correctly populate `status`
            status: MediaStatus::Consumed,
            filename: self.filename,
            alt: self.alt,
            size: s.size,
            content_type: s.mime.clone(),
            source_url: match s.source {
                crate::v1::types::TrackSource::Uploaded => None,
                crate::v1::types::TrackSource::Downloaded { source_url } => Some(source_url),
                crate::v1::types::TrackSource::Extracted => None,
                crate::v1::types::TrackSource::Generated => None,
            },
            metadata: match s.info {
                crate::v1::types::MediaTrackInfo::Video(video) => MediaMetadata::Video {
                    width: video.width,
                    height: video.height,
                    duration: video.duration,
                },
                crate::v1::types::MediaTrackInfo::Audio(audio) => MediaMetadata::Audio {
                    duration: audio.duration,
                },
                crate::v1::types::MediaTrackInfo::Image(image) => MediaMetadata::Image {
                    width: image.width,
                    height: image.height,
                },
                crate::v1::types::MediaTrackInfo::Thumbnail(image) => MediaMetadata::Image {
                    width: image.width,
                    height: image.height,
                },
                crate::v1::types::MediaTrackInfo::TimedText(_) => MediaMetadata::File,
                crate::v1::types::MediaTrackInfo::Text(_) => MediaMetadata::Text,
                crate::v1::types::MediaTrackInfo::Mixed(mixed) => match s.mime.parse() {
                    Ok(s) => match s.ty().as_str() {
                        "video" => MediaMetadata::Video {
                            width: mixed.width.unwrap_or_default(),
                            height: mixed.height.unwrap_or_default(),
                            duration: mixed.width.unwrap_or_default(),
                        },
                        "audio" => MediaMetadata::Audio {
                            duration: mixed.width.unwrap_or_default(),
                        },
                        _ => MediaMetadata::File,
                    },
                    Err(_) => MediaMetadata::File,
                },
                crate::v1::types::MediaTrackInfo::Other => MediaMetadata::File,
            },
            user_id: None,
            deleted_at: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,
        }
    }
}

impl Into<Media> for V1MediaWithAdmin {
    fn into(self) -> Media {
        let user_id = self.user_id;
        let deleted_at = self.deleted_at;
        Media {
            user_id: Some(user_id),
            deleted_at,
            ..self.inner.into()
        }
    }
}
