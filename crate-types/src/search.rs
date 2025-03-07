use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{RoomId, TagId, ThreadId, UserId};

// TODO(#256): fancier searching
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

    /// Only return messages in these rooms. Defaults to all rooms.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub room_id: Vec<RoomId>,

    /// Only return messages in these threads. Defaults to all threads.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub thread_id: Vec<ThreadId>,

    /// Only return messages from these users. Defaults to all threads.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub user_id: Vec<UserId>,

    /// Only return messages that have these features. Defaults to returning all messages.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub features_message: Vec<SearchMessageFeatures>,

    /// Only return messages from threads that have these features. Defaults to searching all threads.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub features_thread: Vec<SearchThreadFeatures>,

    /// Only return messages from rooms that have these features. Defaults to searching all rooms.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub features_room: Vec<SearchRoomFeatures>,

    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub order_by: SearchOrder,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchOrder {
    #[default]
    /// Return the newest matching items first
    Newest,

    /// Return the oldest matching items first
    Oldest,

    /// Return the most relevant matching items first
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

    /// Has an embed
    Embed,

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
    /// Include threads you aren't joined to
    All,

    /// Is pinned
    Pinned,

    /// Include messages from muted threads. Explicitly providing `room_id` or `thread_id` overrides this.
    Muted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchRoomFeatures {
    /// Is a direct message room
    Dm,

    /// Is not a direct message room (overrides Dm)
    NotDm,

    /// Include muted rooms. Explicitly providing `room_id` or `thread_id` overrides this.
    Muted,

    /// Include discoverable public rooms
    Public,
}

// TODO(#76): thread searching
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SearchThreadsRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub query: String,

    /// Only return threads that have these features. Defaults to searching all threads.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub features_thread: Vec<SearchThreadFeatures>,

    /// Only return threads from rooms that have these features. Defaults to searching all rooms.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub features_room: Vec<SearchRoomFeatures>,

    /// Only return threads in these rooms. Defaults to all rooms.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub room_id: Vec<RoomId>,

    /// Only return threads with these tags.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub tag_id: Vec<TagId>,

    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub order_by: SearchOrder,
}

// TODO(#77): room searching
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SearchRoomsRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub query: String,

    /// Only return rooms that have these features. Defaults to searching all rooms.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub features_room: Vec<SearchRoomFeatures>,

    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub order_by: SearchOrder,
}
