use serde::{Deserialize, Serialize};

use crate::{CustomEmojiId, MediaId, RoomId, UserId};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// WARN: this is an *extreme* work in progress
// at this point in time, custom emoji is still very tentative. i'm still not
// sure if i'll implement custom emoji or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum Emoji {
    Custom(EmojiCustom),
    Unicode { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EmojiCustom {
    pub id: CustomEmojiId,
    pub name: String,
    pub creator_id: UserId,
    pub owner: EmojiOwner,
    pub animated: bool,
    pub media_id: MediaId,
    // restrict: Option<Vec<RoleId | UserId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "owner")]
pub enum EmojiOwner {
    /// an emoji owned by a room
    Room { room_id: RoomId },

    /// an emoji owned by the user that creator_id points to
    User,
}
