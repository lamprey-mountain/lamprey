use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{RoomId, ThreadId, UserId};

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
}

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

    /// Only return threads in these rooms. Defaults to all rooms.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub room_id: Vec<RoomId>,

    /// Only return threads with these parents. Defaults to all threads.
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub parent_id: Vec<ThreadId>,
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
}
