use crate::ast::Ast;
use crate::render::MarkdownReader;

/// Returns the input markdown string byte-for-byte without any transformations.
///
/// This reader is useful when you want to preserve the original markdown source
/// or when you need to access the raw input text.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast, IdentityReader};
///
/// let parser = Parser::default();
/// let parsed = parser.parse("**hello** world");
/// let ast = Ast::new(parsed);
/// let reader = IdentityReader;
///
/// let result = reader.read(&ast);
/// assert_eq!(result, "**hello** world");
/// ```
pub struct IdentityReader;

impl IdentityReader {
    /// Read the original markdown source from the AST.
    pub fn read(&self, ast: &Ast) -> String {
        ast.source().to_string()
    }
}

impl MarkdownReader for IdentityReader {
    fn read(&self, ast: &Ast) -> String {
        self.read(ast)
    }
}
