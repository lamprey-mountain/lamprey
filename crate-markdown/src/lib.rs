//! A markdown parser library with support for:
//! - Inline formatting: **bold**, *italic*, ~~strikethrough~~, `code`
//! - Links: [text](url), <https://example.com>, https://example.com
//! - Mentions: @uuid, <@uuid>
//! - Custom emoji: <:name:uuid> or <a:name:uuid>
//! - Block elements: headers, lists, blockquotes, code blocks
//! - Escape sequences: \*, \[, \\, etc.
//! - Incremental editing with tree reuse

pub mod ast;
pub mod parser;
pub mod render;

// Query module is not yet implemented
// mod query;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use ast::{
    AngleBracketLink, Ast, AstNode, Autolink, BlockQuote, CodeBlock, Document, Emoji, Emphasis,
    Escape, Header, InlineCode, Link, List, ListItem, Mention, Paragraph, Strikethrough, Strong,
};
pub use parser::{Edit, ParseOptions, Parsed, Parser, SyntaxKind, TokenKind};
pub use render::{IdentityReader, MarkdownReader, PlainTextReader, StripEmojiReader};
