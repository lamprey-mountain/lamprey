//! Renderers for converting markdown syntax trees to different output formats.
//!
//! # Example
//! ```
//! use lamprey_markdown::{Parser, Ast};
//! use lamprey_markdown::renderer::{Renderer, MarkdownRenderer, PlaintextRenderer};
//!
//! let parser = Parser::default();
//! let ast = Ast::new(parser.parse("**hello** world"));
//!
//! // Render as markdown (identity - preserves source)
//! let markdown = MarkdownRenderer.render(&ast.syntax());
//! assert_eq!(markdown, "**hello** world");
//!
//! // Render as plain text (strips formatting)
//! let text = PlaintextRenderer.render(&ast.syntax());
//! assert!(text.contains("hello"));
//! assert!(text.contains("world"));
//! ```

use rowan::{NodeOrToken, SyntaxNode};

use crate::parser::MyLang;

/// A renderer that converts a markdown syntax tree to a specific output format.
pub trait Renderer {
    /// The output type produced by this renderer.
    type Output;

    /// Render a syntax node to the output format.
    fn render(&self, node: &SyntaxNode<MyLang>) -> Self::Output;
}

/// Renders a syntax tree back to markdown.
///
/// This is an identity renderer - it returns the original source text
/// byte-for-byte. If transformations have been applied, it returns the
/// transformed markdown.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast};
/// use lamprey_markdown::renderer::{Renderer, MarkdownRenderer};
///
/// let parser = Parser::default();
/// let ast = Ast::new(parser.parse("**hello** world"));
///
/// let markdown = MarkdownRenderer.render(&ast.syntax());
/// assert_eq!(markdown, "**hello** world");
/// ```
pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    type Output = String;

    fn render(&self, node: &SyntaxNode<MyLang>) -> Self::Output {
        node.text().to_string()
    }
}

/// Renders a syntax tree to plain text by stripping all formatting.
///
/// This renderer removes all markdown formatting (bold, italic, links, etc.)
/// and returns only the text content.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast};
/// use lamprey_markdown::renderer::{Renderer, PlaintextRenderer};
///
/// let parser = Parser::default();
/// let ast = Ast::new(parser.parse("**hello** *world*"));
///
/// let text = PlaintextRenderer.render(&ast.syntax());
/// assert!(text.contains("hello"));
/// assert!(text.contains("world"));
/// assert!(!text.contains("**"));
/// assert!(!text.contains("*"));
/// ```
pub struct PlaintextRenderer;

impl Renderer for PlaintextRenderer {
    type Output = String;

    fn render(&self, node: &SyntaxNode<MyLang>) -> Self::Output {
        // Collect all text tokens, skipping formatting markers
        collect_plaintext(node)
    }
}

/// Recursively collect plaintext from a syntax node.
fn collect_plaintext(node: &SyntaxNode<MyLang>) -> String {
    use crate::parser::SyntaxKind;

    let mut result = String::new();

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(tok) => {
                // Skip delimiter/marker tokens, keep text
                match tok.kind() {
                    SyntaxKind::Text => {
                        result.push_str(tok.text());
                    }
                    // Skip all markers and delimiters
                    _ => {}
                }
            }
            NodeOrToken::Node(child_node) => {
                // Recurse into child nodes
                match child_node.kind() {
                    // For inline code and code blocks, keep the content
                    SyntaxKind::InlineCode | SyntaxKind::CodeBlock => {
                        result.push_str(&collect_code_content(&child_node));
                    }
                    // For emoji, just use the name
                    SyntaxKind::Emoji => {
                        if let Some(name) = extract_emoji_name(&child_node) {
                            result.push_str(&format!(":{}:", name));
                        }
                    }
                    // For autolinks and angle bracket links, extract the URL
                    SyntaxKind::Autolink | SyntaxKind::AngleBracketLink => {
                        result.push_str(&extract_link_url(&child_node));
                    }
                    // For everything else, recurse
                    _ => {
                        result.push_str(&collect_plaintext(&child_node));
                    }
                }
            }
        }
    }

    result
}

/// Extract URL from an autolink or angle bracket link node.
fn extract_link_url(node: &SyntaxNode<MyLang>) -> String {
    use crate::parser::SyntaxKind;

    for child in node.children() {
        if child.kind() == SyntaxKind::LinkDestination {
            return child.text().to_string();
        }
    }

    // Fallback: return full text
    node.text().to_string()
}

/// Extract code content from inline code or code blocks.
fn collect_code_content(node: &SyntaxNode<MyLang>) -> String {
    use crate::parser::SyntaxKind;

    let mut result = String::new();

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(tok) => {
                if tok.kind() == SyntaxKind::Text {
                    result.push_str(tok.text());
                }
            }
            NodeOrToken::Node(child_node) => {
                result.push_str(&collect_code_content(&child_node));
            }
        }
    }

    result
}

/// Extract emoji name from an emoji node.
fn extract_emoji_name(node: &SyntaxNode<MyLang>) -> Option<String> {
    use crate::parser::SyntaxKind;

    for child in node.children() {
        if child.kind() == SyntaxKind::EmojiName {
            return Some(child.text().to_string());
        }
    }

    // Fallback: find text that's not a UUID
    for child in node.children_with_tokens() {
        if let NodeOrToken::Token(tok) = child {
            let text = tok.text();
            if text.len() != 36 || text.chars().filter(|c| *c == '-').count() != 4 {
                if text != "<" && text != ">" && text != ":" && text != "a" {
                    return Some(text.to_string());
                }
            }
        }
    }

    None
}
