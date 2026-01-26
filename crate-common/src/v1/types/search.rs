#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    Channel, ChannelId, ChannelType, Message, RoleId, RoomId, RoomMember, TagId, ThreadMember,
    User, UserId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SearchMessageRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[serde(default)]
    pub query: Option<String>,

    /// Only return messages in these rooms. Defaults to all rooms.
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub room_id: Vec<RoomId>,

    /// Only return messages in these channels. Defaults to all channels.
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub channel_id: Vec<ChannelId>,

    /// Only return messages from these users. Defaults to all users.
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub user_id: Vec<UserId>,

    /// Only return messages that have an attachment of any type
    pub has_attachment: Option<bool>,

    /// Only return messages that have an attachment of type image/*
    pub has_image: Option<bool>,

    /// Only return messages that have an attachment of type audio/*
    pub has_audio: Option<bool>,

    /// Only return messages that have an attachment of type video/*
    pub has_video: Option<bool>,

    /// Only return messages that have a link
    pub has_link: Option<bool>,

    /// Only return messages that have an embed
    pub has_embed: Option<bool>,

    // /// Only return messages that have an associated thread
    // // maybe not as useful due to the upcoming thread search endpoint
    // pub has_thread: Option<bool>,
    /// Only return pinned (or unpinned) messages
    pub pinned: Option<bool>,

    /// Only return messages that have links from these domains
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub link_hostnames: Vec<String>,

    /// Only return messages that mention these users
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub mentions_users: Vec<UserId>,

    /// Only return messages that mention these roles
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub mentions_roles: Vec<RoleId>,

    /// Only return messages that mentions everyone
    pub mentions_everyone: Option<bool>,

    #[cfg(feature = "feat_search_ordering")]
    /// The key to start paginating from. Not inclusive. Optional.
    pub from: Option<MessageId>,

    /// The key to stop paginating at. Not inclusive. Optional.
    #[cfg(feature = "feat_search_ordering")]
    pub to: Option<MessageId>,

    /// The order to return messages in
    pub order: SearchMessageOrder,

    /// The maximum number of items to return.
    #[cfg(feature = "feat_search_ordering")]
    // TODO: min 0, max 1024, default 100
    pub limit: Option<u16>,

    #[cfg(any())]
    /// whether to include results from nsfw channels
    pub include_nsfw: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SearchChannelsRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[serde(default)]
    pub query: Option<String>,

    /// Only return threads in these rooms. Defaults to all rooms.
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub room_id: Vec<RoomId>,

    /// Only return threads in these channels. Defaults to all channels.
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub parent_id: Vec<ChannelId>,

    /// Only return threads with these tags.
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub tag_id: Vec<TagId>,

    /// Only return archived (or unarchived) threads
    pub archived: Option<bool>,

    /// Only return removed (or not removed) threads
    pub removed: Option<bool>,

    /// only return channels of these types
    #[serde(default, rename = "type")]
    #[cfg_attr(feature = "validator", validate(length(max = 32)))]
    pub ty: Vec<ChannelType>,

    #[cfg(feature = "feat_search_ordering")]
    /// The key to start paginating from. Not inclusive. Optional.
    pub from: Option<MessageId>,

    /// The key to stop paginating at. Not inclusive. Optional.
    #[cfg(feature = "feat_search_ordering")]
    pub to: Option<MessageId>,

    /// The order to return channels in
    #[cfg(feature = "feat_search_ordering")]
    pub order: SearchChannelsOrder,

    /// The maximum number of items to return.
    #[cfg(feature = "feat_search_ordering")]
    // TODO: min 0, max 1024, default 100
    pub limit: Option<u16>,

    #[cfg(any())]
    /// whether to include results from nsfw channels
    pub include_nsfw: bool,
}

// TODO(#77): room searching
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchMessageOrder {
    Relevancy,
    Newest,
    Oldest,
}

#[cfg(feature = "feat_search_ordering")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SearchChannelsOrder {
    Relevancy,
    CreatedNewest,
    CreatedOldest,
    ActivityNewest,
    ActivityOldest,
    ArchiveNewest,
    ArchiveOldest,
}

// TODO: return extra data with search response
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SearchMessageResponse {
    pub messages: Vec<Message>,
    pub users: Vec<User>,
    pub threads: Vec<Channel>,
    pub room_members: Vec<RoomMember>,
    pub thread_members: Vec<ThreadMember>,
    pub total: u64,
}
