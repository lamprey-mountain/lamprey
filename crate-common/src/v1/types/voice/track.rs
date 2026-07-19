use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    UserId,
    voice::{MediaKind, Mid},
};

/// a unique identifier for a media track
///
/// `TrackId`s are server assigned and unique inside each active call.
// TODO: value type u64?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(value_type = String))]
pub struct TrackId(pub u64);

/// the metadata for a track
// TODO: rename to TrackMetadata
#[record]
pub struct TrackMetadata2 {
    /// whether this track is for audio or video
    pub kind: MediaKind,

    /// key to group tracks together into streams
    ///
    /// more or les identical to ssrc but easier to manage client side
    pub key: TrackKey,

    // /// simulcasting layers, only applicable for video
    // #[serde(default, skip_serializing_if = "Vec::is_empty")]
    // pub layers: Vec<TrackLayer>,
    /// whisper config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whisper: Option<TrackWhisper>,
}

// webrtc rids dont need to be in the signalling protocol
// #[record]
// pub struct TrackLayer {
//     pub encoding: TrackEncoding,
// }

/// track whispering config
///
/// only send media from this track to these users
#[record]
pub struct TrackWhisper {
    // TODO: max length
    pub user_ids: Vec<UserId>,
}

/// a mapping from a mid to track metadata
///
/// sent during client offer
#[record]
pub struct TrackCreate {
    #[serde(flatten)]
    pub inner: TrackMetadata2,
    pub mid: Mid,
}

/// mapping from a track id to local mid
///
/// sent by the server during an offer
#[record]
pub struct TrackMapping {
    pub mid: Mid,
    pub id: TrackId,
}

/// an announcement for a track along with its metadata
#[record]
pub struct TrackAnnouncement {
    #[serde(flatten)]
    pub inner: TrackMetadata2,
    pub id: TrackId,
}

/// an update to the list of subscribed tracks
///
/// required to get user video (camera) and screenshare video/audio.
///
/// ## implicit tracks
///
/// Some tracks are subscribed to automatically. Adding or removing these tracks does nothing. Tracks are implicitly subscribed to if they are:
///
/// - audio tracks from key `user`
/// - whisper tracks (TODO)
// NOTE: maybe i should allow adding/removing implicit tracks?
#[record]
pub struct SubscriptionUpdate {
    /// subscribe to these tracks
    pub add: Vec<TrackId>,

    /// unsubscribe from these tracks
    pub remove: Vec<TrackId>,
}

/// which stream a track is associated with
///
/// generally there will be one video track and one audio track per stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
// TODO: rename to MediaKey?
pub enum TrackKey {
    /// media from the user (microphone, camera)
    User,

    /// a screenshare
    Screen,

    /// an unknown track type
    #[serde(untagged)]
    Other(String),
}

#[cfg(feature = "utoipa")]
mod _u {
    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{RefOr, schema::Schema},
        schema,
    };

    use super::*;

    impl ToSchema for TrackKey {}

    impl PartialSchema for TrackKey {
        fn schema() -> RefOr<Schema> {
            RefOr::T(Schema::Object(
                schema!(String)
                    .title(Some("String"))
                    .description(Some("which stream a track is associated with"))
                    .examples(["user", "screen"])
                    .build(),
            ))
        }
    }
}

/// the encoding of the track
// TODO: provide a way to explicitly specify bitrate, framerate, resolution instead of only providing presets?
// TODO: remove this?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackEncoding {
    /// pass the source video through untouched
    Source,

    /// full hd
    Full,

    /// barely usable
    Reduced,

    /// low bandwidth, for thumbnails
    Thumbnail,
}

// impl TrackEncoding {
//     /// get the rid for this encoding
//     pub fn rid(&self) -> Rid {
//         match self {
//             TrackEncoding::Source => Rid::new("s"),
//             TrackEncoding::Full => Rid::new("f"),
//             TrackEncoding::Reduced => Rid::new("r"),
//             TrackEncoding::Thumbnail => Rid::new("t"),
//         }
//     }
// }
