use lamprey_macros::{Diff, record};

use crate::v1::types::{EmojiId, MediaId, RoomId, UserId};

/// a custom emoji
#[record]
#[derive(PartialEq, Eq)]
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

    /// whether this emoji is animated
    pub animated: bool,

    pub media_id: MediaId,
    // /// the pack this emoji is from
    // pub pack_id: Option<RoomId>,
}

/// minimal data for a custom emoji
///
/// only contains what is needed to render an emoji
#[record]
#[derive(PartialEq, Eq)]
pub struct EmojiCustomMinimal {
    pub id: EmojiId,
    pub name: String,
    pub animated: bool,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "owner")]
pub enum EmojiOwner {
    /// an emoji owned by a room
    Room { room_id: RoomId },

    /// an emoji owned by the user that creator_id points to
    User,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct EmojiCustomCreate {
    #[validate(length(min = 2, max = 32), custom(function = "validate_emoji_name"))]
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

#[record]
#[derive(PartialEq, Eq, Diff)]
pub struct EmojiCustomPatch {
    #[validate(length(min = 2, max = 32), custom(function = "validate_emoji_name"))]
    pub name: Option<String>,
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct EmojiSearchQuery {
    pub query: String,
}

impl EmojiCustom {
    /// get the id of the room this custom emoji is in, if known
    pub fn room_id(&self) -> Option<RoomId> {
        match &self.owner {
            Some(EmojiOwner::Room { room_id }) => Some(*room_id),
            _ => None,
        }
    }
}
