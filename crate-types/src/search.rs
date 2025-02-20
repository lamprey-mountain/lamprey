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
    /// Has attachment of any type
    Attachment,

    /// Has attachment of type image/*
    Image,

    /// Has attachment of type audio/*
    Audio,

    /// Has attachment of type video/*
    Video,

    /// Has a hyperlink
    Link,

    /// Is pinned
    Pinned,

    /// Include messages from ignored users. By default these are filtered out.
    Ignored,

    /// Include messages from ignored users. By default these are filtered out. Implicitly includes `Ignored`.
    Blocked,

    /// Only return messages from unmuted threads and rooms. Explicitly providing `room_id` and `thread_id` overrides this.
    NotMuted,
}
