#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{RoomId, emoji::EmojiCustomMinimal, misc::binary::Binary};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PackInstallation {
    pub pack_id: RoomId,
}

/// room metadata for pack rooms
// TODO: add `pack: Option<PackInfo>` to Room
// NOTE: do i want to rename this to `Pack`?
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PackInfo {
    /// the license of this pack
    // NOTE: do i want to reuse redex::License
    pub license: Option<String>,
}

/// a serialized pack
///
/// can be used to import/export pack data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PackSnapshot {
    /// the room's name
    ///
    /// ignored during import
    pub name: String,

    /// the room's description
    ///
    /// ignored during import
    pub description: Option<String>,

    /// the room's license
    ///
    /// ignored during import, ui should show a warning if the license doesnt match
    pub license: Option<String>,

    /// the emojis in this pack
    pub emojis: Vec<PackSnapshotEmoji>,
    // NOTE: i may add stickers/sounds later
    // pub stickers: Vec<Sticker>,
    // pub sounds: Vec<Sound>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PackImport {
    /// the emojis in this pack
    pub emojis: Vec<PackSnapshotEmoji>,
    // NOTE: i may add stickers/sounds later
    // pub stickers: Vec<Sticker>,
    // pub sounds: Vec<Sound>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PackSnapshotEmoji {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: EmojiCustomMinimal,

    /// base64 image data for this emoji
    // NOTE: 33554432 = 32Mib, unsure if this is correct?
    pub data: Binary<33554432>,
}
