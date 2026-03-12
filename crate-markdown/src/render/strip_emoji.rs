use rowan::NodeOrToken;

use lamprey_common::v1::types::EmojiId;
use uuid::Uuid;

use crate::{ast::Ast, parser::SyntaxNode as ParserSyntaxNode, render::MarkdownReader};

/// A reader that wraps another reader and filters out disallowed custom emoji.
///
/// This reader filters out custom emoji (`:name:uuid:`) that are not in the
/// allowed list. All other text content is preserved.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast, StripEmojiReader};
/// use lamprey_common::v1::types::EmojiId;
/// use uuid::uuid;
///
/// let allowed = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
/// let parser = Parser::default();
/// let parsed = parser.parse("hello :smile:12345678-1234-1234-1234-123456789abc: world");
/// let ast = Ast::new(parsed);
/// let reader = StripEmojiReader::new(allowed);
///
/// let result = reader.read(&ast);
/// // Allowed emoji is preserved
/// assert!(result.contains("smile"));
/// ```
pub struct StripEmojiReader<R = ()> {
    pub inner: R,
    pub allowed: Vec<EmojiId>,
}

impl StripEmojiReader {
    /// Create a new StripEmojiReader with the allowed emoji list.
    pub fn new(allowed: Vec<EmojiId>) -> Self {
        StripEmojiReader { inner: (), allowed }
    }

    /// Read text from an AST, filtering out disallowed emoji.
    pub fn read(&self, ast: &Ast) -> String {
        let syntax = ast.syntax();
        collect_with_emoji_filter(&syntax, &self.allowed)
    }
}

impl<R: MarkdownReader> MarkdownReader for StripEmojiReader<R> {
    fn read(&self, ast: &Ast) -> String {
        // Filter emoji from the AST directly
        let syntax = ast.syntax();
        collect_with_emoji_filter(&syntax, &self.allowed)
    }
}

/// Collect text from syntax tree, filtering out disallowed emoji
fn collect_with_emoji_filter(node: &ParserSyntaxNode, allowed: &[EmojiId]) -> String {
    let mut result = String::new();
    collect_with_emoji_filter_impl(node, allowed, &mut result);
    result
}

fn collect_with_emoji_filter_impl(
    node: &ParserSyntaxNode,
    allowed: &[EmojiId],
    result: &mut String,
) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(child_node) => {
                match child_node.kind() {
                    crate::parser::SyntaxKind::Emoji => {
                        // Check if this emoji is allowed
                        let emoji_id = extract_emoji_id(&child_node);
                        if let Some(id) = emoji_id {
                            if allowed.contains(&id) {
                                // Include this emoji - output the emoji syntax
                                result.push(':');
                                // Get emoji name
                                for grandchild in child_node.children_with_tokens() {
                                    if let NodeOrToken::Node(name_node) = grandchild {
                                        if name_node.kind() == crate::parser::SyntaxKind::EmojiName
                                        {
                                            for token in name_node.children_with_tokens() {
                                                if let NodeOrToken::Token(t) = token {
                                                    result.push_str(t.text());
                                                }
                                            }
                                        }
                                    }
                                }
                                result.push(':');
                            }
                            // If not allowed, skip it entirely
                        }
                    }
                    _ => {
                        collect_with_emoji_filter_impl(&child_node, allowed, result);
                    }
                }
            }
            NodeOrToken::Token(token) => {
                result.push_str(token.text());
            }
        }
    }
}

/// Extract the emoji UUID from an Emoji node
fn extract_emoji_id(node: &ParserSyntaxNode) -> Option<EmojiId> {
    for child in node.children_with_tokens() {
        if let NodeOrToken::Token(token) = child {
            let text = token.text();
            // Try to parse as UUID
            if let Ok(uuid) = Uuid::parse_str(text) {
                return Some(EmojiId::from(uuid));
            }
        }
    }
    None
}
