use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::emoji::Emoji;

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
