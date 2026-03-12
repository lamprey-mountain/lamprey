//! Renderers for converting parsed markdown to different output formats.
//!
//! # Example
//! ```
//! use lamprey_markdown::{Parser, Ast, PlainTextReader};
//!
//! let parser = Parser::default();
//! let parsed = parser.parse("**hello** world");
//! let ast = Ast::new(parsed);
//!
//! // Get plain text (strips formatting)
//! let reader = PlainTextReader::new();
//! let text = reader.read(&ast);
//! assert!(text.contains("hello"));
//! assert!(text.contains("world"));
//! ```

pub use identity::IdentityReader;
pub use plain::PlainTextReader;
pub use strip_emoji::StripEmojiReader;

mod identity;
mod plain;
mod strip_emoji;

/// Trait for reading/rendering an AST in different ways.
///
/// Each reader operates independently on the AST. For composition,
/// use the event iterator API:
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast};
/// use lamprey_markdown::events::EventFilter;
///
/// let parser = Parser::default();
/// let ast = Ast::new(parser.parse("**hello** world"));
///
/// // Use event iterators for composition
/// let text: String = ast.events()
///     .map(|e| e.text())
///     .collect();
/// ```
pub trait MarkdownReader {
    /// Read the AST and produce output string.
    fn read(&self, ast: &Ast) -> String;
}

use crate::ast::Ast;
