//! Abstract Syntax Tree types for parsed markdown.
//!
//! The `Ast` struct wraps a parsed document and provides access to the syntax tree.

use crate::parser::{Parsed, SyntaxNode};

/// A reference to a span of text. Indexes are in bytes. Start is inclusive, end is not.
/// We use u32 for wasm compatibility.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// A parsed markdown document.
///
/// Wraps the syntax tree and original source text, providing convenient access methods.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast};
///
/// let parser = Parser::default();
/// let parsed = parser.parse("**hello** world");
/// let ast = Ast::new(parsed);
///
/// // Access the syntax tree
/// let tree = ast.syntax();
///
/// // Access the original source
/// let source = ast.source();
/// assert_eq!(source, "**hello** world");
/// ```
#[derive(Debug, Clone)]
pub struct Ast {
    parsed: Parsed,
}

impl Ast {
    /// Create a new Ast from a parsed document.
    pub fn new(parsed: Parsed) -> Self {
        Self { parsed }
    }

    /// Get the syntax tree root node.
    pub fn syntax(&self) -> SyntaxNode {
        self.parsed.syntax()
    }

    /// Get the original source text.
    pub fn source(&self) -> &str {
        self.parsed.source()
    }
}

// Future extensions (commented out for now):
// pub enum AstBlock { Header, Paragraph, }
// pub struct AstInline { Text, Syntax, Bold { value: Vec<AstInlineA> }, }
// impl Ast { pub fn strip_emoji(), get_mention_ids(), render() }
