use std::ops::Deref;

use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

/// a unique identifier for a media track
///
/// mids are local to each client/sfu pair. corresponds to a transceiver in webrtc.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(value_type = String))]
pub struct Mid(pub [u8; 16]);

impl Mid {
    pub fn new(s: &str) -> Self {
        let mut arr = [b' '; 16];
        let bytes = s.as_bytes();
        let len = bytes.len().min(16);
        arr[..len].copy_from_slice(&bytes[..len]);
        Self(arr)
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("").trim_end()
    }
}

impl Deref for Mid {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Debug for Mid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mid({})", self.as_str())
    }
}

#[cfg(feature = "serde")]
mod _s {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for Mid {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(self.as_str())
        }
    }

    impl<'de> Deserialize<'de> for Mid {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let s = String::deserialize(deserializer)?;
            Ok(Mid::new(&s))
        }
    }
}

impl Deref for SessionDescription {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for IceCandidate {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// the kind of media this track is for
#[record]
#[derive(Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Video,
    Audio,
}

/// the kind of keyframe to request
#[record]
#[derive(Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeyframeRequestKind {
    /// just joined a stream, needs a keyframe for initial rendering
    Fir,

    /// lost some data, need a keyframe to recover
    Pli,
}
