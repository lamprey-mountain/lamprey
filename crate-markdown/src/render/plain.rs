use crate::ast::Ast;
use crate::events::Event;
use crate::render::MarkdownReader;

/// A reader that converts markdown to plain text by stripping formatting.
///
/// This reader removes all formatting markers (bold, italic, links, etc.) and
/// returns only the text content. Escape sequences are processed so that
/// `\*` becomes `*` in the output.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast, PlainTextReader};
///
/// let parser = Parser::default();
/// let parsed = parser.parse("**hello** *world*");
/// let ast = Ast::new(parsed);
///
/// let reader = PlainTextReader::new();
/// let result = reader.read(&ast);
/// assert!(result.contains("hello"));
/// assert!(result.contains("world"));
/// ```
pub struct PlainTextReader;

impl PlainTextReader {
    /// Create a new PlainTextReader.
    pub fn new() -> Self {
        PlainTextReader
    }

    /// Read plain text from an AST using event iteration.
    pub fn read(&self, ast: &Ast) -> String {
        ast.events()
            .filter_map(|event| {
                match event {
                    Event::Text(t) => Some(t),
                    Event::Code(c) => Some(c),
                    // Skip Start/End tags, they're formatting
                    _ => None,
                }
            })
            .collect()
    }
}

impl Default for PlainTextReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownReader for PlainTextReader {
    fn read(&self, ast: &Ast) -> String {
        self.read(ast)
    }
}
