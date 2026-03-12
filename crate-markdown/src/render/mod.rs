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
//! let plain = PlainTextReader::new();
//! let text = plain.read(&ast);
//! // Note: formatting markers are stripped but text content remains
//! assert!(text.contains("hello"));
//! assert!(text.contains("world"));
//! ```

pub use identity::IdentityReader;
pub use plain::PlainTextReader;
pub use strip_emoji::StripEmojiReader;

mod identity;
mod plain;
mod strip_emoji;
// mod html; // NOTE: html isn't needed for now

/// Trait for reading/rendering an AST in different ways.
///
/// This trait allows readers to be composed and chained together.
///
/// # Example
/// ```ignore
/// // The trait methods allow chaining readers, but each reader operates
/// // on the AST directly. For composition, create wrapper types.
/// use lamprey_markdown::{Parser, Ast, PlainTextReader, StripEmojiReader};
/// use lamprey_common::v1::types::EmojiId;
///
/// let parser = Parser::default();
/// let parsed = parser.parse("**hello** :emoji:uuid:");
/// let ast = Ast::new(parsed);
///
/// // Use readers independently
/// let plain = PlainTextReader::new();
/// let text = plain.read(&ast);
/// ```
pub trait MarkdownReader: Sized {
    /// Read the AST and produce output string.
    fn read(&self, ast: &Ast) -> String;

    /// Wrap this reader with a PlainTextReader.
    fn plain(self) -> PlainTextReader<Self> {
        PlainTextReader(self)
    }

    /// Wrap this reader with a StripEmojiReader.
    fn strip_emoji(self, allowed: Vec<EmojiId>) -> StripEmojiReader<Self> {
        StripEmojiReader {
            inner: self,
            allowed,
        }
    }
}

use crate::ast::Ast;
use lamprey_common::v1::types::EmojiId;
