#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{util::Diff, EmojiId, MediaId, RoomId, UserId};

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
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmojiCustomCreate {
    #[cfg_attr(
        feature = "validator",
        validate(length(min = 2, max = 32), custom(function = "validate_emoji_name"))
    )]
    pub name: String,
    pub animated: bool,
    pub media_id: MediaId,
}

/// validate a custom emoji name
#[cfg(feature = "validator")]
fn validate_emoji_name(name: &str) -> Result<(), validator::ValidationError> {
    if name.contains(' ') {
        let mut err = validator::ValidationError::new("invalid_emoji_name");
        err.add_param("message".into(), &"emoji name cannot contain spaces");
        return Err(err);
    }

    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let mut err = validator::ValidationError::new("invalid_emoji_name");
        err.add_param(
            "message".into(),
            &"emoji name can only contain alphanumeric characters and underscores",
        );
        return Err(err);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmojiCustomPatch {
    #[cfg_attr(
        feature = "validator",
        validate(length(min = 2, max = 32), custom(function = "validate_emoji_name"))
    )]
    pub name: Option<String>,
}

impl Diff<EmojiCustom> for EmojiCustomPatch {
    fn changes(&self, other: &EmojiCustom) -> bool {
        self.name.changes(&other.name)
    }
}
