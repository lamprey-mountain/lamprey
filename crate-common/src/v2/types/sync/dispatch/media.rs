use lamprey_macros::record;

use crate::v2::types::media::Media;
use crate::v2::types::{MediaId, SessionId};

/// something happened with a piece of media
#[record]
pub struct DispatchMedia {
    pub media_id: MediaId,

    // /// the room this webhook belongs to, if any
    // pub room_id: Option<RoomId>,

    // /// the channel this webhook belongs to
    // pub channel_id: ChannelId,
    #[serde(flatten)]
    pub inner: DispatchMediaInner,
}

#[record]
#[serde(tag = "type")]
pub enum DispatchMediaInner {
    /// A piece of media has processed and is now in the `Uploaded` state.
    MediaProcessed {
        // NOTE: should i use user_id instead? so apps can eg. have a worker that streams and prints all media it creates?
        session_id: SessionId,

        media: Box<Media>,
    },

    MediaUpdate {
        media: Box<Media>,
    },
}
