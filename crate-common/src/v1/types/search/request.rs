#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::search::Order;

// TODO: make query not an Option?

/// generic search request struct
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SearchRequest {
    /// the full text search query.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub query: Option<String>,

    /// sort order (ascending/descending)
    #[cfg_attr(feature = "serde", serde(default = "Order::descending"))]
    pub sort_order: Order,

    /// the maximum number of items to return
    #[cfg_attr(feature = "serde", serde(default = "default_limit"))]
    #[cfg_attr(feature = "utoipa", schema(default = 100, minimum = 0, maximum = 1024))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 1024)))]
    pub limit: u16,

    /// the number of items to skip before returning
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "utoipa", schema(default = 0, minimum = 0, maximum = 65535))]
    #[cfg_attr(feature = "validator", validate(range(min = 0, max = 65535)))]
    pub offset: u16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageSearchRequest {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SearchRequest,

    /// field to sort by
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: MessageSearchOrderField,

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
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SearchRequest,

    /// field to sort by
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: ChannelSearchOrderField,

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

    /// sort by channel name
    Name,

    /// sort by channel id
    Id,
}

/// room search request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomSearchRequest {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SearchRequest,

    /// what order to return results in
    #[cfg_attr(feature = "serde", serde(default))]
    pub order: RoomSearchOrderField,

    /// field to sort by
    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: RoomSearchOrderField,
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

    /// sort by room id
    Id,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UserSearchRequest {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SearchRequest,

    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: UserSearchOrderField,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserSearchOrderField {
    #[default]
    Name,
    Created,
    Registered,
    Id,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaSearchRequest {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SearchRequest,

    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: MediaSearchOrderField,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaSearchOrderField {
    #[default]
    Created,
    Name,
    Id,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AuditLogSearchRequest {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: SearchRequest,

    #[cfg_attr(feature = "serde", serde(default))]
    pub sort_field: AuditLogSearchOrderField,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AuditLogSearchOrderField {
    #[default]
    Created,
}

const fn default_limit() -> u16 {
    100
}
