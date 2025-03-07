use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::{emoji::Emoji, UserId};

/// the total reaction counts for all emoji
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ReactionCounts(Vec<ReactionCount>);

/// the total reaction counts for an emoji
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ReactionCount {
    pub emoji: Emoji,
    pub count: u64,

    #[serde(rename = "self")]
    pub self_reacted: bool,
}

/// a reaction from a user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Reaction {
    pub emoji: Emoji,
    pub user_id: UserId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct ReactionListParams {
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub emoji: Option<Emoji>,
}
