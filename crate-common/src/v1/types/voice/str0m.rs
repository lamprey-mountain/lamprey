use str0m::media::KeyframeRequestKind as KeyframeRequestKindStr0m;
use str0m::media::MediaKind as MediaKindStr0m;
use str0m::media::Mid as MidStr0m;
use str0m::media::Rid as RidStr0m;

use crate::v1::types::voice::{KeyframeRequestKind, MediaKind, Mid, Rid};

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

// TODO: skip string conversions
impl From<Mid> for MidStr0m {
    fn from(value: Mid) -> Self {
        MidStr0m::from(value.0.to_string().as_str())
    }
}

impl From<MidStr0m> for Mid {
    fn from(value: MidStr0m) -> Self {
        Mid(uuid::Uuid::parse_str(&value.to_string()).unwrap_or_default())
    }
}

impl From<Rid> for RidStr0m {
    fn from(value: Rid) -> Self {
        RidStr0m::from_array(value.0.to_le_bytes())
    }
}

impl From<RidStr0m> for Rid {
    fn from(value: RidStr0m) -> Self {
        let s: &str = &value;
        Rid(s.parse().unwrap_or_default())
    }
}
