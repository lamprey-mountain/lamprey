use lamprey_macros::record;

use crate::v1::types::{reaction::ReactionKeyField, search::Order};

// TODO: make query not an Option?

/// generic search request struct
#[record]
pub struct SearchRequest {
    /// the full text search query.
    #[schema(required = false, min_length = 1, max_length = 2048)]
    #[validate(length(min = 1, max = 2048))]
    #[serde(default)]
    pub query: Option<String>,

    /// sort order (ascending/descending)
    #[serde(default = "Order::descending")]
    pub sort_order: Order,

    /// the maximum number of items to return
    #[serde(default = "default_limit")]
    #[schema(default = 100, minimum = 0, maximum = 1024)]
    #[validate(range(min = 0, max = 1024))]
    pub limit: u16,

    /// the number of items to skip before returning
    #[serde(default)]
    #[schema(default = 0, minimum = 0, maximum = 65535)]
    #[validate(range(min = 0, max = 65535))]
    pub offset: u16,
}

#[record]
pub struct MessageSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    /// field to sort by
    #[serde(default)]
    pub sort_field: MessageSearchOrderField,

    /// whether to include results from nsfw channels
    pub include_nsfw: Option<bool>,
}

/// which field to order message search results by
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum MessageSearchOrderField {
    /// sort by creation time
    #[default]
    Created,

    /// sort by relevancy
    Relevancy,
}

#[record]
pub struct ChannelSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    /// field to sort by
    #[serde(default, flatten)]
    pub sort_field: ChannelSearchOrderField,

    /// whether to include nsfw channels
    pub include_nsfw: Option<bool>,
}

/// which field to order channel search results by
#[record]
#[derive(Default, PartialEq, Eq)]
#[serde(tag = "field")]
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

    /// sort by channel name
    Name,

    /// sort by channel id
    Id,

    /// sort by score
    Score,

    /// sort by number of reactions
    Reactions { reaction: ReactionKeyField },
}

/// room search request
#[record]
pub struct RoomSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    /// what order to return results in
    #[serde(default)]
    pub order: RoomSearchOrderField,

    /// field to sort by
    #[serde(default)]
    pub sort_field: RoomSearchOrderField,
}

/// which field to order room search results by
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum RoomSearchOrderField {
    /// sort by number of members
    #[default]
    Members,

    /// sort by creation time
    Created,

    /// sort by room name
    Name,

    /// sort by room id
    Id,
}

#[record]
pub struct UserSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: UserSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum UserSearchOrderField {
    #[default]
    Name,
    Created,
    Registered,
    Id,
}

#[record]
pub struct MediaSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: MediaSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum MediaSearchOrderField {
    #[default]
    Created,
    Name,
    Id,
}

#[record]
pub struct AuditLogSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: AuditLogSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum AuditLogSearchOrderField {
    #[default]
    Created,
}

#[record]
pub struct EverythingSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: EverythingSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum EverythingSearchOrderField {
    #[default]
    Id,
}

const fn default_limit() -> u16 {
    100
}
