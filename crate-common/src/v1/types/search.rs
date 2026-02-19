#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "feat_search_ordering")]
use crate::v1::types::MessageId;
use crate::v1::types::{
    misc::Time, Channel, ChannelId, ChannelType, Message, RoleId, RoomId, RoomMember, TagId,
    ThreadMember, User, UserId,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageSearchRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub query: Option<String>,

    /// Only return messages in these rooms. Defaults to all rooms.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub room_id: Vec<RoomId>,

    /// Only return messages in these channels. Defaults to all channels.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub channel_id: Vec<ChannelId>,

    /// Only return messages from these users. Defaults to all users.
    #[cfg_attr(feature = "serde", serde(default))]
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

    /// Only return messages that have an associated thread
    // NOTE: maybe not as useful due to the channel/thread search endpoint
    pub has_thread: Option<bool>,

    /// Only return pinned (or unpinned) messages
    pub pinned: Option<bool>,

    /// Only return messages that have links from these domains
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub link_hostnames: Vec<String>,

    /// Only return messages that mention these users
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub mentions_users: Vec<UserId>,

    /// Only return messages that mention these roles
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub mentions_roles: Vec<RoleId>,

    /// Only return messages that mentions everyone
    pub mentions_everyone: Option<bool>,

    /// only include messages ids in this range
    #[cfg_attr(feature = "serde", serde(default))]
    pub message_id: FilterRange<MessageId>,

    /// sort order (ascending/descending)
    #[cfg_attr(feature = "serde", serde(default = "Order::descending"))]
    // return newest by default
    pub sort_order: Order,

    /// field to sort by
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: MessageSearchOrderField,

    /// the maximum number of messages to return
    #[cfg_attr(feature = "serde", serde(default = "default_limit"))]
    #[cfg_attr(feature = "utoipa", schema(default = 100, minimum = 0, maximum = 1024))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 1024)))]
    pub limit: u16,

    /// the number of messages to skip before returning
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "utoipa", schema(default = 0, minimum = 0, maximum = 65535))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 65535)))]
    pub offset: u16,

    /// whether to include results from nsfw channels
    pub include_nsfw: Option<bool>,
}

/// which field to order message search results by
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageSearchOrderField {
    /// sort by creation time
    #[default]
    Created,

    /// sort by relevancy
    Relevancy,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelSearchRequest {
    /// The full text search query. Consider this an implementation detail, but I currently use postgres' [`websearch_to_tsquery`](https://www.postgresql.org/docs/17/textsearch-controls.html#TEXTSEARCH-PARSING-QUERIES) function.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub query: Option<String>,

    /// Only return threads in these rooms. Defaults to all rooms.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub room_id: Vec<RoomId>,

    /// Only return threads in these channels. Defaults to all channels.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub parent_id: Vec<ChannelId>,

    /// Only return threads with these tags.
    // maybe allow configuring tag matching (any/all)
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub tag_id: Vec<TagId>,

    /// Only return archived (or unarchived) threads
    pub archived: Option<bool>,

    /// Only return removed (or not removed) threads
    pub removed: Option<bool>,

    /// only return channels of these types
    #[cfg_attr(feature = "serde", serde(default, rename = "type"))]
    #[cfg_attr(feature = "validator", validate(length(max = 32)))]
    pub ty: Vec<ChannelType>,

    /// only include channel ids in this range
    #[cfg_attr(feature = "serde", serde(default))]
    pub message_id: FilterRange<MessageId>,

    /// sort order (ascending/descending)
    #[cfg_attr(feature = "serde", serde(default = "Order::descending"))]
    // return newest by default
    pub sort_order: Order,

    /// field to sort by
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: ChannelSearchOrderField,

    /// the maximum number of channels to return
    #[cfg_attr(feature = "serde", serde(default = "default_limit"))]
    #[cfg_attr(feature = "utoipa", schema(default = 100, minimum = 0, maximum = 1024))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 1024)))]
    pub limit: u16,

    /// the number of channels to skip before returning
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "utoipa", schema(default = 0, minimum = 0, maximum = 65535))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 65535)))]
    pub offset: u16,

    /// whether to include nsfw channels
    pub include_nsfw: Option<bool>,
}

/// which field to order channel search results by
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ChannelSearchOrderField {
    /// sort by creation time
    #[default]
    Created,

    /// sort by relevancy
    Relevancy,

    /// sort by last activity time
    Activity,

    /// sort by archival time
    Archived,
}

/// room search request
// TODO(#77): room searching
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomSearchRequest {
    /// what order to return results in
    #[cfg_attr(feature = "serde", serde(default))]
    pub order: RoomSearchOrderField,

    /// filter by room name, description, and id
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub query: Option<String>,

    /// only return rooms created in this range
    #[cfg_attr(feature = "serde", serde(default))]
    pub created_at: FilterRange<Time>,

    /// filter by owner id
    ///
    /// admin only
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    #[cfg_attr(feature = "utoipa", schema(max_items = 128))]
    pub owner_id: Vec<UserId>,

    /// filter by deletion timestamp range
    ///
    /// admin only
    pub deleted_at: FilterRange<Time>,

    /// filter by archival timestamp range
    ///
    /// admin only
    pub archived_at: FilterRange<Time>,

    /// filter by quarantine status
    ///
    /// admin only
    pub quarantined: Option<bool>,

    /// filter by if this room is public
    ///
    /// required to be true for non-admins
    pub public: Option<bool>,

    /// sort order (ascending/descending)
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_order: Order,

    /// field to sort by
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: RoomSearchOrderField,

    /// the maximum number of messages to return
    #[cfg_attr(feature = "serde", serde(default = "default_limit"))]
    #[cfg_attr(feature = "utoipa", schema(default = 100, minimum = 0, maximum = 1024))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 1024)))]
    pub limit: u16,

    /// the number of channels to skip before returning
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "utoipa", schema(default = 0, minimum = 0, maximum = 65535))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 65535)))]
    pub offset: u16,
}

/// which field to order room search results by
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomSearchOrderField {
    /// sort by number of members
    #[default]
    Members,

    /// sort by creation time
    Created,

    /// sort by room name
    Name,
}

// TODO: return extra data with search response
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSearch {
    /// the ids of the matched messages
    pub results: Vec<MessageId>,

    /// all relevant messages (eg. messages that a result replied to)
    pub messages: Vec<Message>,

    /// the authors of the messages
    pub users: Vec<User>,

    /// threads the messages are in
    pub threads: Vec<Channel>,

    /// room members objects for each author, if they exist
    pub room_members: Vec<RoomMember>,

    /// relevant thread member objects
    ///
    /// - one for each (message author, thread) tuple
    /// - one for each thread the requesting user is a member of
    pub thread_members: Vec<ThreadMember>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub approximate_total: u64,
}

// pub struct ChannelSearch {}
// pub struct UserSearch {}
// pub struct RoomSearch {}

/// filter results to only this range
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum FilterRange<T> {
    /// only return values in this range
    Range { min: Option<T>, max: Option<T> },

    /// any non-null value
    #[cfg_attr(feature = "serde", serde(rename = "any"))]
    #[default]
    Any,
}

/// what order to return items in
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Order {
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "asc"))]
    Ascending,

    #[cfg_attr(feature = "serde", serde(rename = "desc"))]
    Descending,
}

impl Order {
    pub fn descending() -> Order {
        Order::Descending
    }
}

const fn default_limit() -> u16 {
    100
}
