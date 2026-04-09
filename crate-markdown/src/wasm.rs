//! WASM bindings for the markdown parser.
//!
//! This module exposes two use cases:
//! 1. **SolidJS rendering** — one-shot `text → events` (JSON) for client-side rendering
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

/// Result of parsing, containing events and tokens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub events: Vec<WasmEvent>,
    pub tokens: Vec<WasmToken>,
    pub source_length: u32,
}

// ============ One-shot API (SolidJS rendering use case) ============

/// Parse markdown and return events + tokens as JSON.
///
/// This is the primary API for SolidJS rendering: call this once with your
/// markdown, and you get back a structured representation of the document.
///
/// # Arguments
/// * `markdown` - The markdown source to parse
///
/// # Returns
/// JSON string containing `ParseResult` with events and tokens
#[wasm_bindgen]
pub fn parse_markdown(markdown: &str) -> String {
    let parser = Parser::default();
    let parsed = parser.parse(markdown);
    let ast = Ast::new(parsed.clone());

    let events = collect_events(&ast);
    let tokens = collect_tokens(&parsed);

    let result = ParseResult {
        events,
        tokens,
        source_length: markdown.len() as u32,
    };

    serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
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
/// ProseMirror syntax highlighting. Call `parse` once, then `edit`
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
    /// Returns a JSON array of `{ kind, start, end, text }` objects.
    /// Use `start` and `end` (byte offsets) to apply decorations in ProseMirror.
    pub fn tokens(&self) -> String {
        let tokens = collect_tokens(&self.parsed);
        serde_json::to_string(&tokens).unwrap_or_else(|_e| "[]".to_string())
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

    /// Apply an incremental edit and return the updated tokens as JSON.
    ///
    /// Convenience method that calls `edit` and returns tokens directly.
    pub fn edit_and_tokens(&mut self, delete_start: u32, delete_end: u32, insert: &str) -> String {
        let parser = Parser::default();
        let edit = Edit {
            delete: TextRange::new(delete_start.into(), delete_end.into()),
            insert,
        };
        self.parsed = parser.edit(&self.parsed, edit);
        let tokens = collect_tokens(&self.parsed);
        serde_json::to_string(&tokens).unwrap_or_else(|_e| "[]".to_string())
    }
}

// ============ Transformation API ============

/// Strip disallowed emoji from markdown, converting them to `:name:` format.
///
/// # Arguments
/// * `markdown` - The markdown source
/// * `allowed_emojis` - JSON array of allowed emoji UUID strings
///
/// # Returns
/// Transformed markdown with disallowed emoji stripped
#[wasm_bindgen]
pub fn strip_emoji(markdown: &str, allowed_emojis: &str) -> String {
    let allowed_ids: Vec<Uuid> = serde_json::from_str::<Vec<String>>(allowed_emojis)
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
    MarkdownRenderer.render(&transformed_node)
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
