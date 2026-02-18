#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Diff, EmojiId, MediaId, RoomId, UserId};

// WARN: this is an *extreme* work in progress

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EmojiCustom {
    pub id: EmojiId,
    pub name: String,

    /// the user who created this emoji
    ///
    /// not returned unless
    /// - the owner is a room and you're in the room this emoji is in
    /// - the owner is a user and you're the creator
    pub creator_id: Option<UserId>,

    /// the place where this emoji exists
    ///
    /// not returned unless
    /// - the owner is a room and you're in the room this emoji is in
    /// - the owner is a user and you're the creator
    pub owner: Option<EmojiOwner>,

    pub animated: bool,

    pub media_id: MediaId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "owner"))]
pub enum EmojiOwner {
    /// an emoji owned by a room
    Room { room_id: RoomId },

    /// an emoji owned by the user that creator_id points to
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EmojiCustomCreate {
    // TODO(#862): enforce emoji naming conventions (ie. no spaces)
    pub name: String,
    pub animated: bool,
    pub media_id: MediaId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EmojiCustomPatch {
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub name: Option<String>,
}

impl Diff<EmojiCustom> for EmojiCustomPatch {
    fn changes(&self, other: &EmojiCustom) -> bool {
        self.name.changes(&other.name)
    }
}
