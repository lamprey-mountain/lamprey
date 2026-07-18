//! str0m conversions

use str0m::media::KeyframeRequestKind as SKeyframeRequestKind;
use str0m::media::MediaKind as SMediaKind;
use str0m::media::Mid as SMid;

use crate::v1::types::voice::{KeyframeRequestKind, MediaKind, Mid};

impl From<MediaKind> for SMediaKind {
    fn from(value: MediaKind) -> Self {
        match value {
            MediaKind::Video => SMediaKind::Video,
            MediaKind::Audio => SMediaKind::Audio,
        }
    }
}

impl From<SMediaKind> for MediaKind {
    fn from(value: SMediaKind) -> Self {
        match value {
            SMediaKind::Video => MediaKind::Video,
            SMediaKind::Audio => MediaKind::Audio,
        }
    }
}

impl From<KeyframeRequestKind> for SKeyframeRequestKind {
    fn from(value: KeyframeRequestKind) -> Self {
        match value {
            KeyframeRequestKind::Fir => SKeyframeRequestKind::Fir,
            KeyframeRequestKind::Pli => SKeyframeRequestKind::Pli,
        }
    }
}

impl From<SKeyframeRequestKind> for KeyframeRequestKind {
    fn from(value: SKeyframeRequestKind) -> Self {
        match value {
            SKeyframeRequestKind::Fir => KeyframeRequestKind::Fir,
            SKeyframeRequestKind::Pli => KeyframeRequestKind::Pli,
        }
    }
}

impl From<Mid> for SMid {
    fn from(value: Mid) -> Self {
        SMid::from_array(value.0)
    }
}

impl From<SMid> for Mid {
    fn from(value: SMid) -> Self {
        Mid::new(&value.to_string())
    }
}
