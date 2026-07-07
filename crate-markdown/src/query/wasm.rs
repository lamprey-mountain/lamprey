use crate::{
    ast::inline::{CustomEmoji, CustomEmojiData, MentionData, UnicodeEmoji},
    parser::Parsed,
    prelude::*,
    query::{Decoration, QueryableExt},
};
use serde::Serialize;

#[derive(Serialize)]
struct LinkDto {
    href: String,
    text: Option<String>,
    span: Span,
}

#[derive(Serialize)]
struct MentionDto {
    text: String,
    #[serde(flatten)]
    data: MentionData,
    span: Span,
}

#[derive(Serialize)]
struct EmojiDto {
    text: String,
    #[serde(flatten)]
    kind: EmojiDtoKind,
    span: Span,
}

#[derive(Serialize)]
#[serde(tag = "kind")]
enum EmojiDtoKind {
    Custom(CustomEmojiData),
    Unicode,
}

#[derive(Serialize)]
struct HeaderDto {
    level: u8,
    text: String,
    span: Span,
}

#[wasm_bindgen]
impl Parsed {
    /// Get decorations within an optional span range
    #[wasm_bindgen(js_name = "decorations")]
    pub fn js_decorations(&self, start: Option<Len>, end: Option<Len>) -> Result<JsValue, JsValue> {
        let span = match (start, end) {
            (Some(s), Some(e)) => Some(Span::from((s, e))),
            _ => None,
        };

        let decos: Vec<Decoration> = self.tree_clone().iter_decorations(span).collect();

        serde_wasm_bindgen::to_value(&decos).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "links")]
    pub fn js_links(&self) -> Result<JsValue, JsValue> {
        let links: Vec<LinkDto> = self
            .tree_clone()
            .iter_links()
            .map(|l| LinkDto {
                href: l.href(),
                text: Some(
                    l.children()
                        .map(|c| c.syntax().to_string())
                        .collect::<String>(),
                ),
                span: l.syntax().text_range().into(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&links).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "mentions")]
    pub fn js_mentions(&self) -> Result<JsValue, JsValue> {
        let mentions: Vec<MentionDto> = self
            .tree_clone()
            .iter_mentions()
            .map(|m| MentionDto {
                text: m.text(),
                data: m.parse(),
                span: m.syntax().text_range().into(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&mentions).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "emoji")]
    pub fn js_emoji(&self) -> Result<JsValue, JsValue> {
        let emoji: Vec<EmojiDto> = self
            .tree_clone()
            .iter_emoji()
            .filter_map(|e| {
                let (kind, text, span) = if let Some(custom) = CustomEmoji::cast(e.syntax().clone())
                {
                    (
                        EmojiDtoKind::Custom(custom.parse()),
                        custom.text(),
                        custom.syntax().text_range().into(),
                    )
                } else if let Some(unicode) = UnicodeEmoji::cast(e.syntax().clone()) {
                    (
                        EmojiDtoKind::Unicode,
                        unicode.text(),
                        unicode.syntax().text_range().into(),
                    )
                } else {
                    // NOTE: is this unreachable?
                    return None;
                };
                Some(EmojiDto { text, kind, span })
            })
            .collect();
        serde_wasm_bindgen::to_value(&emoji).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "headers")]
    pub fn js_headers(&self) -> Result<JsValue, JsValue> {
        let headers: Vec<HeaderDto> = self
            .tree_clone()
            .iter_headers()
            .map(|h| HeaderDto {
                level: h.level(),
                text: h
                    .children()
                    .map(|c| c.syntax().to_string())
                    .collect::<String>(),
                span: h.syntax().text_range().into(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&headers).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "onlyEmoji")]
    pub fn js_only_emoji(&self) -> Option<u32> {
        self.tree_clone().only_emoji()
    }
}
