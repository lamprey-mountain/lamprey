use lamprey_macros::record;

use crate::v1::types::{
    Channel, ChannelId, reaction::ReactionKeyField, search::common::SearchRequest,
};

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

#[record]
pub struct ChannelSearch {
    /// the ids of the matched channels
    pub results: Vec<ChannelId>,

    /// the channels
    pub channels: Vec<Channel>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}
