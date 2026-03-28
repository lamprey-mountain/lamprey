use std::collections::HashMap;

use crate::v1::types::media::{MediaV0 as V1Media, MediaV0WithAdmin as V1MediaWithAdmin};
use crate::v2::types::media::{Media, MediaMetadata, MediaStatus};

impl From<Media> for V1Media {
    fn from(val: Media) -> Self {
        V1Media {
            id: val.id,
            filename: val.filename,
            alt: val.alt,
            source: crate::v1::types::MediaTrack {
                info: match val.metadata {
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
                size: val.size,
                mime: val.content_type,
                source: if let Some(source_url) = val.source_url {
                    crate::v1::types::TrackSource::Downloaded { source_url }
                } else {
                    crate::v1::types::TrackSource::Uploaded
                },
            },
        }
    }
}

impl From<V1Media> for Media {
    fn from(val: V1Media) -> Self {
        let s = val.source;
        Media {
            id: val.id,
            // WARNING: the database is going to need to correctly populate `status`
            status: MediaStatus::Consumed,
            filename: val.filename,
            alt: val.alt,
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
            quarantine: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,

            // NOTE: these should probably be populated later?
            links: vec![],
            room_id: None,
            channel_id: None,
            hashes: HashMap::default(),
            strip_exif: false,
        }
    }
}

impl From<V1MediaWithAdmin> for Media {
    fn from(val: V1MediaWithAdmin) -> Self {
        let user_id = val.user_id;
        let deleted_at = val.deleted_at;
        Media {
            user_id: Some(user_id),
            deleted_at,
            ..val.inner.into()
        }
    }
}
