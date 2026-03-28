//! A markdown parser library with support for:
//! - Inline formatting: **bold**, *italic*, ~~strikethrough~~, `code`
//! - Links: [text](url), <https://example.com>, https://example.com
//! - Mentions: @uuid, <@uuid>
//! - Custom emoji: <:name:uuid> or <a:name:uuid>
//! - Block elements: headers, lists, blockquotes, code blocks
//! - Escape sequences: \*, \[, \\, etc.
//! - Incremental editing with tree reuse
//!
//! # Architecture
//!
//! This library uses a transformation-based architecture:
//!
//! 1. **Parse** markdown into a syntax tree using [`Parser`]
//! 2. **Transform** the tree using [`Transformation`] implementations
//! 3. **Render** the tree to output using [`Renderer`] implementations
//!
//! # Example
//! ```
//! use lamprey_markdown::{Parser, Ast};
//! use lamprey_markdown::transformer::{Transformation, Pipeline, StripEmoji};
//! use lamprey_markdown::renderer::{Renderer, MarkdownRenderer};
//!
//! let parser = Parser::default();
//! let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc>"));
//!
//! // Create transformation pipeline
//! let mut pipeline = Pipeline::default();
//! pipeline.add_transform(StripEmoji::new(vec![]));
//!
//! // Apply transformations and render
//! let transformed = pipeline.apply(&ast.syntax());
//! let transformed_node = rowan::SyntaxNode::new_root(transformed);
//! let markdown = MarkdownRenderer.render(&transformed_node);
//! assert!(markdown.contains(":smile:"));
//! ```

pub mod ast;
pub mod events;
pub mod parser;
pub mod render;
pub mod renderer;
pub mod transformer;

// Query module is not yet implemented
// mod query;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use ast::{
    AngleBracketLink,
    Ast,
    AstNode,
    Autolink,
    BlockQuote,
    CodeBlock,
    Document,
    Emoji,
    Emphasis,
    Escape,
    Header,
    InlineCode,
    Link,
    LinkKind,
    // Query types
    LinkRef,
    List,
    ListItem,
    Mention,
    MentionId,
    MentionIds,
    Paragraph,
    Strikethrough,
    Strong,
};
pub use events::{Event, EventFilter, EventIterator, Tag};
pub use parser::{Edit, ParseOptions, Parsed, Parser, SyntaxKind, TokenKind};

// New architecture exports
pub use renderer::{MarkdownRenderer, PlaintextRenderer, Renderer};
pub use transformer::{apply, find_emoji_nodes, Pipeline, StripEmoji, Transformation};

// Legacy exports (deprecated - use new architecture)
#[deprecated(note = "Use new transformer module instead")]
pub use render::{IdentityReader, MarkdownReader, PlainTextReader, StripEmojiReader};
