//! Transformations for modifying markdown syntax trees.
//!
//! This module provides a trait-based system for transforming markdown ASTs.
//! Transformations are applied bottom-up, rebuilding only changed subtrees.
//!
//! # Architecture
//!
//! The transformation system supports both node and token transformations:
//!
//! - **Node transformations**: Replace entire syntax nodes (e.g., strip emoji nodes)
//! - **Token transformations**: Modify leaf tokens (e.g., uppercase text tokens)
//!
//! Transformations are applied in post-order (children before parents), allowing
//! transformations to work on already-transformed subtrees.
//!
//! # Example
//! ```
//! use lamprey_markdown::{Parser, Ast};
//! use lamprey_markdown::transformer::{Transformation, apply, Pipeline, StripEmoji};
//! use lamprey_common::v1::types::EmojiId;
//! use uuid::uuid;
//!
//! let parser = Parser::default();
//! let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc>"));
//!
//! // Create a transformation pipeline
//! let mut pipeline = Pipeline::default();
//! pipeline.add_transform(StripEmoji::new(vec![]));
//!
//! // Apply transformations
//! let transformed = pipeline.apply(&ast.syntax());
//! ```

use lamprey_common::v1::types::EmojiId;
use rowan::{GreenNode, GreenNodeBuilder, GreenToken, NodeOrToken, SyntaxNode, SyntaxToken};
use uuid::Uuid;

use crate::parser::{MyLang, SyntaxKind};

/// A transformation to apply over a markdown syntax tree.
///
/// Transformations are applied bottom-up, meaning children are transformed
/// before their parents. This allows transformations to work on already-transformed
/// subtrees.
///
/// The trait provides two methods:
/// - `transform_node`: Called for each syntax node (default: no change)
/// - `transform_token`: Called for each token (default: no change)
///
/// Implement only the methods you need; the other will use the default no-op implementation.
pub trait Transformation {
    /// Transform a syntax node, returning a new GreenNode if changed.
    ///
    /// Return `None` to keep the node unchanged (but still process its children/tokens).
    /// Return `Some(green_node)` to replace the entire node (children won't be processed).
    fn transform_node(&self, _node: &SyntaxNode<MyLang>) -> Option<GreenNode> {
        None
    }

    /// Transform a token, returning a new GreenToken if changed.
    ///
    /// Return `None` to keep the token unchanged.
    /// Return `Some(green_token)` to replace the token.
    fn transform_token(&self, _token: &SyntaxToken<MyLang>) -> Option<GreenToken> {
        None
    }
}

/// Walk bottom-up, rebuilding only changed subtrees.
///
/// This function recursively transforms children and tokens first (post-order),
/// then gives the transformation a chance to modify the parent node.
pub fn apply(t: &dyn Transformation, node: &SyntaxNode<MyLang>) -> GreenNode {
    // First, check if the transformation wants to replace this node entirely
    if let Some(replacement) = t.transform_node(node) {
        return replacement;
    }

    // Otherwise, transform children and tokens, then rebuild this node
    let new_children: Vec<_> = node
        .children_with_tokens()
        .map(|child| match child {
            NodeOrToken::Node(n) => NodeOrToken::Node(apply(t, &n)),
            NodeOrToken::Token(tok) => {
                // Transform the token
                let transformed = t
                    .transform_token(&tok)
                    .unwrap_or_else(|| tok.green().to_owned());
                NodeOrToken::Token(transformed)
            }
        })
        .collect();

    // Rebuild node with transformed children
    GreenNode::new(node.kind().into(), new_children)
}

/// Apply a transformation to a single token (without processing children).
pub fn apply_token(t: &dyn Transformation, token: &SyntaxToken<MyLang>) -> GreenToken {
    t.transform_token(token)
        .unwrap_or_else(|| token.green().to_owned())
}

/// Compose multiple transformations into a pipeline.
///
/// Transformations are applied in order, with each transformation seeing
/// the output of the previous one.
#[derive(Default)]
pub struct Pipeline {
    transforms: Vec<Box<dyn Transformation>>,
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
        }
    }

    /// Add a transformation to the pipeline.
    pub fn add_transform<T: Transformation + 'static>(&mut self, transform: T) {
        self.transforms.push(Box::new(transform));
    }

    /// Apply all transformations in the pipeline to a node.
    pub fn apply(&self, node: &SyntaxNode<MyLang>) -> GreenNode {
        self.transforms
            .iter()
            .fold(node.green().into_owned(), |acc, t| {
                let current = SyntaxNode::new_root(acc.clone());
                apply(t.as_ref(), &current)
            })
    }
}

impl Transformation for Pipeline {
    fn transform_node(&self, _node: &SyntaxNode<MyLang>) -> Option<GreenNode> {
        // Pipeline itself doesn't transform, it delegates to children
        None
    }

    fn transform_token(&self, _token: &SyntaxToken<MyLang>) -> Option<GreenToken> {
        None
    }
}

/// A transformation that strips disallowed custom emoji.
///
/// This transformation converts disallowed emoji (`<:name:uuid>` or `<a:name:uuid>`)
/// to `:name:` format. Allowed emoji are preserved.
pub struct StripEmoji {
    pub allowed: Vec<EmojiId>,
}

impl StripEmoji {
    /// Create a new StripEmoji transformation with the allowed emoji list.
    pub fn new(allowed: Vec<EmojiId>) -> Self {
        Self { allowed }
    }

    /// Check if an emoji UUID is in the allowed list.
    fn is_allowed(&self, uuid_str: &str) -> bool {
        Uuid::parse_str(uuid_str)
            .ok()
            .map(|uuid| EmojiId::from(uuid))
            .map(|id| self.allowed.contains(&id))
            .unwrap_or(false)
    }
}

impl Transformation for StripEmoji {
    fn transform_node(&self, node: &SyntaxNode<MyLang>) -> Option<GreenNode> {
        if node.kind() != SyntaxKind::Emoji {
            return None;
        }

        // Extract emoji components
        let mut name = None;
        let mut uuid = None;

        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Token(tok) => {
                    if tok.kind() == SyntaxKind::EmojiMarker.into() && tok.text() == "a" {
                        // Animated marker - we don't need to track this since we're stripping
                    } else if tok.kind() == SyntaxKind::Text.into() {
                        let text = tok.text();
                        // Check if it's a UUID (36 chars with 4 dashes)
                        if text.len() == 36 && text.chars().filter(|c| *c == '-').count() == 4 {
                            uuid = Some(text.to_string());
                        }
                    }
                }
                NodeOrToken::Node(n) => {
                    if n.kind() == SyntaxKind::EmojiName {
                        name = Some(n.text().to_string());
                    }
                }
            }
        }

        let (name, uuid) = match (name, uuid) {
            (Some(n), Some(u)) => (n, u),
            _ => return None, // Can't transform without both name and uuid
        };

        if self.is_allowed(&uuid) {
            return None; // Keep allowed emoji unchanged
        }

        // Replace with :name: format
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::Text.into());
        builder.token(SyntaxKind::Text.into(), &format!(":{}:", name));
        builder.finish_node();

        Some(builder.finish())
    }
}

/// A transformation that converts text to uppercase.
///
/// This demonstrates token-level transformation.
pub struct UppercaseText;

impl Transformation for UppercaseText {
    fn transform_token(&self, token: &SyntaxToken<MyLang>) -> Option<GreenToken> {
        if token.kind() != SyntaxKind::Text {
            return None;
        }

        let text = token.text();
        let upper = text.to_uppercase();

        if text == upper {
            return None; // No change needed
        }

        Some(GreenToken::new(SyntaxKind::Text.into(), &upper))
    }
}

/// A transformation that converts text to lowercase.
///
/// This demonstrates token-level transformation.
pub struct LowercaseText;

impl Transformation for LowercaseText {
    fn transform_token(&self, token: &SyntaxToken<MyLang>) -> Option<GreenToken> {
        if token.kind() != SyntaxKind::Text {
            return None;
        }

        let text = token.text();
        let lower = text.to_lowercase();

        if text == lower {
            return None; // No change needed
        }

        Some(GreenToken::new(SyntaxKind::Text.into(), &lower))
    }
}

/// Helper to collect text from a node's tokens
pub fn collect_text(node: &SyntaxNode<MyLang>) -> String {
    node.descendants_with_tokens()
        .filter_map(|n| n.into_token())
        .map(|t| t.text().to_string())
        .collect()
}

/// Find emoji nodes in a syntax tree
pub fn find_emoji_nodes(node: &SyntaxNode<MyLang>) -> Vec<SyntaxNode<MyLang>> {
    node.descendants()
        .filter(|n| n.kind() == SyntaxKind::Emoji)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParseOptions, Parser};
    use crate::Ast;

    #[test]
    fn test_uppercase_text() {
        let parser = Parser::new(ParseOptions::default());
        let ast = Ast::new(parser.parse("hello world"));

        let transformed = apply(&UppercaseText, &ast.syntax());
        let node = SyntaxNode::<MyLang>::new_root(transformed);
        let result = node.text().to_string();

        assert_eq!(result, "HELLO WORLD");
    }

    #[test]
    fn test_lowercase_text() {
        let parser = Parser::new(ParseOptions::default());
        let ast = Ast::new(parser.parse("HELLO WORLD"));

        let transformed = apply(&LowercaseText, &ast.syntax());
        let node = SyntaxNode::<MyLang>::new_root(transformed);
        let result = node.text().to_string();

        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_pipeline_node_and_token_transforms() {
        let parser = Parser::new(ParseOptions::default());
        let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc>"));

        let mut pipeline = Pipeline::new();
        pipeline.add_transform(StripEmoji::new(vec![]));
        pipeline.add_transform(UppercaseText);

        let transformed = pipeline.apply(&ast.syntax());
        let node = SyntaxNode::<MyLang>::new_root(transformed);
        let result = node.text().to_string();

        assert_eq!(result, "HELLO :SMILE:");
    }

    #[test]
    fn test_strip_emoji_with_uppercase() {
        let parser = Parser::new(ParseOptions::default());
        let ast = Ast::new(parser.parse("**hello** <:smile:12345678-1234-1234-1234-123456789abc>"));

        let mut pipeline = Pipeline::new();
        pipeline.add_transform(StripEmoji::new(vec![]));
        pipeline.add_transform(UppercaseText);

        let transformed = pipeline.apply(&ast.syntax());
        let node = SyntaxNode::<MyLang>::new_root(transformed);
        let result = node.text().to_string();

        assert_eq!(result, "**HELLO** :SMILE:");
    }
}
