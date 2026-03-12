use rowan::NodeOrToken;

use crate::{ast::Ast, parser::SyntaxNode as ParserSyntaxNode, render::MarkdownReader};

/// A reader that wraps another reader and strips markdown syntax tokens, returning plain text.
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
/// // Use PlainTextReader directly
/// let reader = PlainTextReader::new();
/// let result = reader.read(&ast);
/// // Formatting markers are stripped, text content remains
/// assert!(result.contains("hello"));
/// assert!(result.contains("world"));
/// ```
pub struct PlainTextReader<R = ()>(pub R);

impl PlainTextReader {
    /// Create a new PlainTextReader with no inner reader (reads directly from AST).
    pub fn new() -> Self {
        PlainTextReader(())
    }

    /// Read plain text from an AST, stripping all markdown formatting.
    pub fn read(&self, ast: &Ast) -> String {
        let syntax = ast.syntax();
        collect_plain_text(&syntax)
    }
}

impl Default for PlainTextReader {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: MarkdownReader> MarkdownReader for PlainTextReader<R> {
    fn read(&self, ast: &Ast) -> String {
        // First get the inner reader's output, then strip formatting
        // For now, just strip directly from AST
        let syntax = ast.syntax();
        collect_plain_text(&syntax)
    }
}

/// Collect plain text from a syntax tree, skipping delimiters and markers
fn collect_plain_text(node: &ParserSyntaxNode) -> String {
    let mut result = String::new();
    collect_plain_text_impl(node, &mut result);
    result
}

fn collect_plain_text_impl(node: &ParserSyntaxNode, result: &mut String) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(child_node) => {
                // Skip delimiter-only nodes but recurse into content nodes
                match child_node.kind() {
                    // Skip delimiter nodes
                    crate::parser::SyntaxKind::StrongDelimiter
                    | crate::parser::SyntaxKind::EmphasisDelimiter
                    | crate::parser::SyntaxKind::StrikethroughDelimiter
                    | crate::parser::SyntaxKind::InlineCodeFence
                    | crate::parser::SyntaxKind::LinkText
                    | crate::parser::SyntaxKind::LinkDestination
                    | crate::parser::SyntaxKind::LinkTitle
                    | crate::parser::SyntaxKind::HeaderMarker
                    | crate::parser::SyntaxKind::ListMarker
                    | crate::parser::SyntaxKind::BlockQuoteMarker
                    | crate::parser::SyntaxKind::CodeBlockFence
                    | crate::parser::SyntaxKind::MentionMarker
                    | crate::parser::SyntaxKind::EmojiMarker => {
                        // Skip these markers entirely
                    }
                    // For Escape nodes, only output the escaped character (not the backslash)
                    crate::parser::SyntaxKind::Escape => {
                        // Recurse but only collect EscapedChar tokens
                        for esc_child in child_node.children_with_tokens() {
                            if let NodeOrToken::Token(t) = esc_child {
                                if t.kind() == crate::parser::SyntaxKind::EscapedChar {
                                    result.push_str(t.text());
                                }
                            }
                        }
                    }
                    // Skip emoji entirely (it's a special element)
                    crate::parser::SyntaxKind::Emoji => {
                        // Skip emoji nodes
                    }
                    // Recurse into content nodes
                    crate::parser::SyntaxKind::InlineCodeContent
                    | crate::parser::SyntaxKind::CodeBlockContent => {
                        collect_plain_text_impl(&child_node, result);
                    }
                    // For other nodes, recurse to get their text content
                    _ => {
                        collect_plain_text_impl(&child_node, result);
                    }
                }
            }
            NodeOrToken::Token(token) => {
                // Include text tokens, but skip backslashes and escape-related tokens
                if token.kind() != crate::parser::SyntaxKind::Escape {
                    result.push_str(token.text());
                }
            }
        }
    }
}
