use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

// use crate::{RoomId, ThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SearchMessageRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub query: String,
    // TODO: fancier searching
    // #[serde(default)]
    // /// Only return messages in these rooms. Defaults to all rooms.
    // room_id: Vec<RoomId>,

    // #[serde(default)]
    // /// Only return messages in these threads. Defaults to all threads.
    // thread_id: Vec<ThreadId>,

    // #[serde(default)]
    // /// Only return messages from these users. Defaults to all threads.
    // user_id: Vec<UserId>,

    // #[serde(default)]
    // /// Only return messages that have these features. Defaults to returning all messages.
    // features: Vec<SearchMessageFeatures>,
    // features_message: Vec<SearchMessageFeatures>,
    //
    // #[serde(default)]
    // /// Only return messages from threads that have these features. Defaults to searching all threads.
    // features_thread: Vec<SearchThreadFeatures>,
    //
    // #[serde(default)]
    // /// Only return messages from rooms that have these features. Defaults to searching all rooms.
    // features_room: Vec<SearchRoomFeatures>,

    // #[serde(default)]
    // order_by: SearchMessageOrder,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchMessageOrder {
    /// Return the oldest matching messages first
    Oldest,

    #[default]
    /// Return the newest matching messages first
    Newest,

    /// Return the most relevant matching messages first
    Relevance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchMessageFeatures {
    /// Has an attachment of any type
    Attachment,

    /// Has an attachment of type image/*
    Image,

    /// Has an attachment of type audio/*
    Audio,

    /// Has an attachment of type video/*
    Video,

    /// Has a link
    Link,

    /// Is pinned
    Pinned,

    /// Include messages from ignored users. By default these are filtered out.
    Ignored,

    /// Include messages from ignored users. By default these are filtered out. Implicitly includes `Ignored`.
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchThreadFeatures {
    /// Is pinned
    Pinned,

    /// Include messages from muted threads. Explicitly providing `room_id` or `thread_id` overrides this.
    Muted,
}

// struct MessageFilter {}
// struct ThreadFilter {}
// struct RoomFilter {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchRoomFeatures {
    /// Is a direct message room
    Dm,

    /// Is not a direct message room (overrides Dm)
    NotDm,

    /// Include messages from muted rooms. Explicitly providing `room_id` or `thread_id` overrides this.
    Muted,
}
