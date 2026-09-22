use crate::prelude::*;

use lamprey_common::v1::types::emoji::{
    EmojiCustom as LampreyEmoji, EmojiCustomMinimal as LampreyEmojiMinimal,
};

/// data about a custom emoji
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct CustomEmojiData {
    pub animated: bool,
    pub name: String,
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub id: Uuid,
}

impl From<LampreyEmoji> for CustomEmojiData {
    fn from(emoji: LampreyEmoji) -> Self {
        Self {
            animated: emoji.animated,
            name: emoji.name,
            id: *emoji.id,
        }
    }
}

impl From<LampreyEmojiMinimal> for CustomEmojiData {
    fn from(emoji: LampreyEmojiMinimal) -> Self {
        Self {
            animated: emoji.animated,
            name: emoji.name,
            id: *emoji.id,
        }
    }
}
