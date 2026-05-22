use str0m::media::KeyframeRequestKind as KeyframeRequestKindStr0m;
use str0m::media::MediaKind as MediaKindStr0m;

use crate::v1::types::voice::{KeyframeRequestKind, MediaKind};

impl From<MediaKind> for MediaKindStr0m {
    fn from(value: MediaKind) -> Self {
        match value {
            MediaKind::Video => MediaKindStr0m::Video,
            MediaKind::Audio => MediaKindStr0m::Audio,
        }
    }
}

impl From<MediaKindStr0m> for MediaKind {
    fn from(value: MediaKindStr0m) -> Self {
        match value {
            MediaKindStr0m::Video => MediaKind::Video,
            MediaKindStr0m::Audio => MediaKind::Audio,
        }
    }
}

impl From<KeyframeRequestKind> for KeyframeRequestKindStr0m {
    fn from(value: KeyframeRequestKind) -> Self {
        match value {
            KeyframeRequestKind::Fir => KeyframeRequestKindStr0m::Fir,
            KeyframeRequestKind::Pli => KeyframeRequestKindStr0m::Pli,
        }
    }
}

impl From<KeyframeRequestKindStr0m> for KeyframeRequestKind {
    fn from(value: KeyframeRequestKindStr0m) -> Self {
        match value {
            KeyframeRequestKindStr0m::Fir => KeyframeRequestKind::Fir,
            KeyframeRequestKindStr0m::Pli => KeyframeRequestKind::Pli,
        }
    }
}
