use crate::{
    ast::inline::{CustomEmojiData, Emoji, MentionData},
    parser::Parsed,
    prelude::*,
    query::{Decoration, QueryableExt},
};
use serde::Serialize;
use tsify::Tsify;

/// Represents a markdown link.
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct LinkItem {
    pub href: String,
    pub text: Option<String>,
    pub span: Span,
}

/// Represents a markdown mention.
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct MentionItem {
    pub text: String,
    #[serde(flatten)]
    pub data: MentionData,
    pub span: Span,
}

/// Represents a markdown emoji.
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi, hashmap_as_object)]
pub struct EmojiItem {
    pub text: String,
    #[serde(flatten)]
    pub kind: EmojiItemKind,
    pub span: Span,
}

#[derive(Tsify, Serialize)]
#[serde(tag = "kind")]
#[tsify(into_wasm_abi)]
pub enum EmojiItemKind {
    Custom(CustomEmojiData),
    Unicode,
}

/// Represents a markdown header.
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct HeaderItem {
    pub level: u8,
    pub text: String,
    pub span: Span,
}

#[wasm_bindgen]
impl Parsed {
    /// Get decorations within an optional span range
    #[wasm_bindgen(js_name = "decorations")]
    pub fn js_decorations(&self, start: Option<Len>, end: Option<Len>) -> Vec<Decoration> {
        let span = match (start, end) {
            (Some(s), Some(e)) => Some(Span::from((s, e))),
            _ => None,
        };

        self.tree().iter_decorations(span).collect()
    }

    /// Get all links
    #[wasm_bindgen(js_name = "links")]
    pub fn js_links(&self) -> Vec<LinkItem> {
        self.tree()
            .iter_links()
            .map(|l| LinkItem {
                href: l.href(),
                text: Some(
                    l.children()
                        .map(|c| c.syntax().to_string())
                        .collect::<String>(),
                ),
                span: l.syntax().text_range().into(),
            })
            .collect()
    }

    /// Get all mentions
    #[wasm_bindgen(js_name = "mentions")]
    pub fn js_mentions(&self) -> Vec<MentionItem> {
        self.tree()
            .iter_mentions()
            .map(|m| MentionItem {
                text: m.text(),
                data: m.parse(),
                span: m.syntax().text_range().into(),
            })
            .collect()
    }

    /// Get all unicode and custom emoji
    #[wasm_bindgen(js_name = "emoji")]
    pub fn js_emoji(&self) -> Vec<EmojiItem> {
        self.tree()
            .iter_emoji()
            .map(|e| {
                let kind = match &e {
                    Emoji::Custom(e) => EmojiItemKind::Custom(e.parse()),
                    Emoji::Unicode(_e) => EmojiItemKind::Unicode,
                };

                let span = e.syntax().text_range().into();

                let text = match &e {
                    Emoji::Custom(e) => e.text(),
                    Emoji::Unicode(e) => e.text(),
                };

                EmojiItem { text, kind, span }
            })
            .collect()
    }

    /// Get all markdown headers
    #[wasm_bindgen(js_name = "headers")]
    pub fn js_headers(&self) -> Vec<HeaderItem> {
        self.tree()
            .iter_headers()
            .map(|h| HeaderItem {
                level: h.level(),
                text: h
                    .children()
                    .map(|c| c.syntax().to_string())
                    .collect::<String>(),
                span: h.syntax().text_range().into(),
            })
            .collect()
    }

    /// Check if this only contains emoji. Returns the number of contained emoji, and null otherwise.
    #[wasm_bindgen(js_name = "onlyEmoji")]
    pub fn js_only_emoji(&self) -> Option<u32> {
        self.tree().only_emoji()
    }
}
