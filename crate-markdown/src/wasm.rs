//! WASM bindings for the markdown parser.
//!
//! This module exposes two use cases:
//! 1. **SolidJS rendering** — one-shot `text → events` (JS objects) for client-side rendering
//! 2. **ProseMirror syntax highlighting** — incremental reparsing with token-level info

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::events::{Event, Tag};
use crate::parser::{Edit, Parsed};
use crate::renderer::{MarkdownRenderer, PlaintextRenderer, Renderer};
use crate::transformer::{Pipeline, StripEmoji};
use crate::{Ast, Parser};
use rowan::{NodeOrToken, TextRange};
use std::str::FromStr;
use uuid::Uuid;

// ============ Serializable types for JS ============

/// A single event emitted during markdown parsing.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WasmEvent {
    Start { tag: String },
    End { tag: String },
    Text { content: String },
    Code { content: String },
    SoftBreak,
    HardBreak,
    Rule,
    Html { content: String },
}

/// A token with its byte range, used for syntax highlighting.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmToken {
    pub kind: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
}

/// A mention entity extracted from the markdown.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MentionEntity {
    pub mention_type: String, // "user" | "role" | "channel"
    pub id: String,
}

/// An emoji entity extracted from the markdown.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiEntity {
    pub animated: bool,
    pub name: String,
    pub id: String,
}

/// A spoiler entity extracted from the markdown.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpoilerEntity {
    /// Byte range of the spoiler content (excluding delimiters)
    pub content_start: u32,
    pub content_end: u32,
}

/// Result of parsing, containing events, tokens, and extracted entities.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmParseResult {
    pub events: Vec<WasmEvent>,
    pub tokens: Vec<WasmToken>,
    pub mentions: Vec<MentionEntity>,
    pub emoji: Vec<EmojiEntity>,
    pub spoilers: Vec<SpoilerEntity>,
    pub source_length: u32,
}

/// Result of an incremental edit, returning updated tokens.
#[allow(dead_code)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmEditResult {
    pub tokens: Vec<WasmToken>,
    pub source_length: u32,
}

// ============ One-shot API (SolidJS rendering use case) ============

/// Parse Markdown and return events + tokens as native JS objects.
///
/// This is the primary API for SolidJS rendering: call this once with your
/// markdown, and you get back a structured representation of the document
/// with extracted entities (mentions, emoji, spoilers).
///
/// # Arguments
/// * `markdown` - The markdown source to parse
///
/// # Returns
/// Native JS object containing events, tokens, and extracted entities
#[wasm_bindgen]
pub fn parse_markdown(markdown: &str) -> Result<JsValue, JsValue> {
    let parser = Parser::default();
    let parsed = parser.parse(markdown);
    let ast = Ast::new(parsed.clone());

    let events = collect_events(&ast);
    let tokens = collect_tokens(&parsed);
    let mentions = extract_mentions(&ast);
    let emoji = extract_emoji(&ast);
    let spoilers = extract_spoilers(&ast);

    let result = WasmParseResult {
        events,
        tokens,
        mentions,
        emoji,
        spoilers,
        source_length: markdown.len() as u32,
    };

    Ok(serde_wasm_bindgen::to_value(&result)?)
}

/// Parse markdown and render to HTML-like markdown string (identity render).
#[wasm_bindgen]
pub fn render_markdown(markdown: &str) -> String {
    let parser = Parser::default();
    let parsed = parser.parse(markdown);
    let ast = Ast::new(parsed);
    MarkdownRenderer.render(&ast.syntax())
}

/// Parse markdown and render to plain text (strip all formatting).
#[wasm_bindgen]
pub fn render_plaintext(markdown: &str) -> String {
    let parser = Parser::default();
    let parsed = parser.parse(markdown);
    let ast = Ast::new(parsed);
    PlaintextRenderer.render(&ast.syntax())
}

// ============ Incremental API (ProseMirror syntax highlighting use case) ============

/// Holds a parsed document for incremental editing.
///
/// Use this when you need to reparse text incrementally, such as for
/// ProseMirror syntax highlighting. Call `new` once, then `edit_and_tokens`
/// for each change.
#[wasm_bindgen]
pub struct WasmParsed {
    parsed: Parsed,
}

#[wasm_bindgen]
impl WasmParsed {
    /// Parse markdown source into a syntax tree.
    ///
    /// # Arguments
    /// * `source` - The markdown source to parse
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str) -> Self {
        let parser = Parser::default();
        let parsed = parser.parse(source);
        Self { parsed }
    }

    /// Get the current source text.
    #[wasm_bindgen(getter)]
    pub fn source(&self) -> String {
        self.parsed.source().to_string()
    }

    /// Get the source length in bytes.
    #[wasm_bindgen(getter)]
    pub fn source_length(&self) -> u32 {
        self.parsed.source().len() as u32
    }

    /// Get all tokens with their byte ranges for syntax highlighting.
    ///
    /// Returns a JS array of `{ kind, start, end, text }` objects.
    /// Use `start` and `end` (byte offsets) to apply decorations in ProseMirror.
    pub fn tokens(&self) -> Result<JsValue, JsValue> {
        let tokens = collect_tokens(&self.parsed);
        Ok(serde_wasm_bindgen::to_value(&tokens)?)
    }

    /// Apply an incremental edit to the document and return the updated result.
    ///
    /// This reuses unchanged portions of the syntax tree for efficiency.
    ///
    /// # Arguments
    /// * `delete_start` - Byte offset where deletion begins (inclusive)
    /// * `delete_end` - Byte offset where deletion ends (exclusive)
    /// * `insert` - Text to insert at the deletion point
    pub fn edit(&self, delete_start: u32, delete_end: u32, insert: &str) -> WasmParsed {
        let parser = Parser::default();
        let edit = Edit {
            delete: TextRange::new(delete_start.into(), delete_end.into()),
            insert,
        };
        let new_parsed = parser.edit(&self.parsed, edit);
        WasmParsed { parsed: new_parsed }
    }

    /// Apply an incremental edit and return the updated tokens as a JS array.
    ///
    /// Convenience method that calls `edit` and returns tokens directly.
    pub fn edit_and_tokens(
        &mut self,
        delete_start: u32,
        delete_end: u32,
        insert: &str,
    ) -> Result<JsValue, JsValue> {
        let parser = Parser::default();
        let edit = Edit {
            delete: TextRange::new(delete_start.into(), delete_end.into()),
            insert,
        };
        self.parsed = parser.edit(&self.parsed, edit);
        let tokens = collect_tokens(&self.parsed);
        Ok(serde_wasm_bindgen::to_value(&tokens)?)
    }
}

// ============ Transformation API ============

/// Strip disallowed emoji from markdown, converting them to `:name:` format.
///
/// # Arguments
/// * `markdown` - The markdown source
/// * `allowed_emojis` - JS array of allowed emoji UUID strings
///
/// # Returns
/// Transformed markdown with disallowed emoji stripped
#[wasm_bindgen]
pub fn strip_emoji(markdown: &str, allowed_emojis: JsValue) -> Result<JsValue, JsValue> {
    let allowed_ids: Vec<Uuid> = serde_wasm_bindgen::from_value::<Vec<String>>(allowed_emojis)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| Uuid::from_str(&s).ok())
        .collect();

    let parser = Parser::default();
    let parsed = parser.parse(markdown);
    let ast = Ast::new(parsed);

    let mut pipeline = Pipeline::new();
    pipeline.add_transform(StripEmoji::new(allowed_ids));

    let transformed = pipeline.apply(&ast.syntax());
    let transformed_node = rowan::SyntaxNode::new_root(transformed);
    let result = MarkdownRenderer.render(&transformed_node);

    Ok(serde_wasm_bindgen::to_value(&result)?)
}

// ============ Internal helpers ============

fn collect_events(ast: &Ast) -> Vec<WasmEvent> {
    let mut events = Vec::new();

    for (event, _span) in ast.events_with_spans() {
        let wasm_event = match event {
            Event::Start(tag) => WasmEvent::Start {
                tag: tag_to_string(&tag),
            },
            Event::End(tag) => WasmEvent::End {
                tag: tag_to_string(&tag),
            },
            Event::Text(t) => WasmEvent::Text {
                content: t.into_owned(),
            },
            Event::Code(c) => WasmEvent::Code {
                content: c.into_owned(),
            },
            Event::SoftBreak => WasmEvent::SoftBreak,
            Event::HardBreak => WasmEvent::HardBreak,
            Event::Rule => WasmEvent::Rule,
            Event::Html(h) => WasmEvent::Html {
                content: h.into_owned(),
            },
            // SourceSpan is metadata, skip it
            Event::SourceSpan(_) => continue,
        };
        events.push(wasm_event);
    }

    events
}

fn tag_to_string(tag: &Tag<'_>) -> String {
    match tag {
        Tag::Paragraph => "paragraph".into(),
        Tag::Header(level) => format!("header_{}", level),
        Tag::List(numbered) => format!("list_{}", if *numbered { "ordered" } else { "bullet" }),
        Tag::ListItem => "list_item".into(),
        Tag::BlockQuote => "blockquote".into(),
        Tag::CodeBlock => "code_block".into(),
        Tag::Strong => "strong".into(),
        Tag::Emphasis => "emphasis".into(),
        Tag::Strikethrough => "strikethrough".into(),
        Tag::Spoiler => "spoiler".into(),
        Tag::InlineCode => "inline_code".into(),
        Tag::Link { .. } => "link".into(),
        Tag::Autolink => "autolink".into(),
        Tag::AngleBracketLink => "angle_bracket_link".into(),
        Tag::Mention => "mention".into(),
        Tag::Emoji { animated, name, .. } => {
            if *animated {
                format!("emoji_animated_{}", name)
            } else {
                format!("emoji_{}", name)
            }
        }
    }
}

fn collect_tokens(parsed: &Parsed) -> Vec<WasmToken> {
    let mut tokens = Vec::new();
    let syntax = parsed.syntax();

    for descendant in syntax.descendants_with_tokens() {
        match descendant {
            NodeOrToken::Node(_) => {}
            NodeOrToken::Token(tok) => {
                let range = tok.text_range();
                tokens.push(WasmToken {
                    kind: format!("{:?}", tok.kind()),
                    start: range.start().into(),
                    end: range.end().into(),
                    text: tok.text().to_string(),
                });
            }
        }
    }

    tokens
}

fn extract_mentions(ast: &Ast) -> Vec<MentionEntity> {
    use crate::parser::SyntaxKind;
    let mut mentions = Vec::new();

    for node in ast.syntax().descendants() {
        match node.kind() {
            SyntaxKind::Mention => {
                if let Some(id) = extract_mention_id(&node) {
                    mentions.push(MentionEntity {
                        mention_type: "user".into(),
                        id,
                    });
                }
            }
            SyntaxKind::MentionRole => {
                if let Some(id) = extract_mention_id(&node) {
                    mentions.push(MentionEntity {
                        mention_type: "role".into(),
                        id,
                    });
                }
            }
            SyntaxKind::MentionChannel => {
                if let Some(id) = extract_mention_id(&node) {
                    mentions.push(MentionEntity {
                        mention_type: "channel".into(),
                        id,
                    });
                }
            }
            _ => {}
        }
    }

    mentions
}

fn extract_mention_id(node: &crate::parser::SyntaxNode) -> Option<String> {
    use crate::parser::SyntaxKind;
    for child in node.children() {
        if child.kind() == SyntaxKind::Text {
            let text = child.text().to_string();
            // UUID is 36 chars with 4 dashes
            if text.len() == 36 && text.chars().filter(|c| *c == '-').count() == 4 {
                return Some(text);
            }
        }
    }
    None
}

fn extract_emoji(ast: &Ast) -> Vec<EmojiEntity> {
    use crate::parser::SyntaxKind;
    let mut emoji = Vec::new();

    for node in ast.syntax().descendants() {
        if node.kind() == SyntaxKind::Emoji {
            if let Some(entity) = extract_emoji_entity(&node) {
                emoji.push(entity);
            }
        }
    }

    emoji
}

fn extract_emoji_entity(node: &crate::parser::SyntaxNode) -> Option<EmojiEntity> {
    use crate::parser::SyntaxKind;
    let mut name = None;
    let mut uuid = None;
    let mut animated = false;

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(tok) => {
                if tok.kind() == SyntaxKind::EmojiMarker && tok.text() == "a" {
                    animated = true;
                } else if tok.kind() == SyntaxKind::Text {
                    let text = tok.text();
                    if text.len() == 36 && text.chars().filter(|c| *c == '-').count() == 4 {
                        uuid = Some(text.to_string());
                    }
                }
            }
            NodeOrToken::Node(n) => {
                if n.kind() == SyntaxKind::EmojiName {
                    name = Some(n.text().to_string());
                }
            }
        }
    }

    match (name, uuid) {
        (Some(n), Some(u)) => Some(EmojiEntity {
            animated,
            name: n,
            id: u,
        }),
        _ => None,
    }
}

fn extract_spoilers(ast: &Ast) -> Vec<SpoilerEntity> {
    use crate::parser::SyntaxKind;
    let mut spoilers = Vec::new();

    for node in ast.syntax().descendants() {
        if node.kind() == SyntaxKind::Spoiler {
            // The spoiler node structure is:
            // SpoilerDelimiter("||") + inline content + SpoilerDelimiter("||")
            // We want the content range (excluding delimiters)
            let children: Vec<_> = node.children_with_tokens().collect();
            if children.len() >= 3 {
                // Skip first delimiter, get start of content
                // Skip last delimiter, get end of content
                let content_start = children[1].text_range().start().into();
                let content_end = children[children.len() - 2].text_range().end().into();
                spoilers.push(SpoilerEntity {
                    content_start,
                    content_end,
                });
            }
        }
    }

    spoilers
}
