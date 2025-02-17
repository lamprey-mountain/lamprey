#![allow(unused)] // TEMP

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub streams: Vec<Stream>,
    pub format: Format,
}

#[derive(Debug, Deserialize)]
pub struct Format {
    pub duration: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Stream {
    pub index: u64,
    pub codec_name: String,
    pub codec_type: MediaType,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub disposition: Disposition,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Disposition {
    pub default: u8,
    pub attached_pic: u8,
}

#[derive(Default, Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
// https://ffmpeg.org/doxygen/7.0/avutil_8h_source.html#l00199
pub enum MediaType {
    /// Usually treated as Data
    Unknown,

    Video,

    Audio,

    Data,

    /// Opaque data information usually continuous
    Subtitle,

    /// Opaque data information usually sparse
    Attachment,

    #[default]
    Nb,
}

impl Metadata {
    pub fn get_main(&self, ty: MediaType) -> Option<&Stream> {
        self.streams
            .iter()
            .find(|i| i.codec_type == ty && i.disposition.default == 1)
            .or_else(|| self.streams.iter().find(|i| i.codec_type == ty))
    }

    /// get the default or first video stream
    pub fn get_main_video(&self) -> Option<&Stream> {
        self.get_main(MediaType::Video)
    }

    pub fn width(&self) -> Option<u64> {
        self.get_main_video().and_then(|v| v.width)
    }

    pub fn height(&self) -> Option<u64> {
        self.get_main_video().and_then(|v| v.height)
    }

    /// in milliseconds, for video/audio
    pub fn duration(&self) -> Option<f64> {
        match &self.format.duration {
            Some(s) => {
                let secs: f64 = s.parse().ok()?;
                Some(secs * 1000.)
            }
            None => None,
        }
    }

    pub fn is_video(&self) -> bool {
        self.get_main_video().is_some()
    }

    pub fn get_thumb_stream(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .find(|i| {
                i.codec_type == MediaType::Attachment
                    && i.disposition.default == 1
                    && i.disposition.attached_pic == 1
            })
            .or_else(|| self.get_main(MediaType::Attachment))
            .or_else(|| self.get_main(MediaType::Video))
    }
}
