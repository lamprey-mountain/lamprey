use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    reaction::ReactionKeyField,
    search::{ChannelSearchOrderField, Order},
};

#[record]
#[derive(Default)]
pub struct ForumSorting {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub reactions: Vec<DefaultForumReaction>,
    pub default_sort: ForumSort,
}

#[record]
pub struct DefaultForumReaction {
    pub reaction: ReactionKeyField,

    /// how to weight this reaction for scoring
    #[serde(default)]
    pub weight: f64,
}

#[record]
#[derive(Default)]
pub struct ForumSort {
    /// what order to return posts in
    #[serde(default)]
    pub order: Order,

    /// only include posts younger than this time
    ///
    /// in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u64>,

    #[serde(flatten)]
    pub kind: ChannelSearchOrderField,
}

/// how to display posts in a forum
#[record]
#[derive(Default, PartialEq, Eq)]
pub enum ForumLayout {
    /// display posts as a list of cards
    #[default]
    Card,

    /// display posts as a compact list
    Compact,

    /// display posts as an image gallery
    Gallery,
}
