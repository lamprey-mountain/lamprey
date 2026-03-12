//! Abstract Syntax Tree types for parsed markdown.
//!
//! This module provides typed wrappers around the raw `rowan::SyntaxNode` tree,
//! offering a type-safe API for working with parsed markdown documents.
//!
//! # Example
//! ```
//! use lamprey_markdown::{Parser, Ast};
//! use lamprey_markdown::ast::{AstNode, Paragraph, Strong};
//!
//! let parser = Parser::default();
//! let parsed = parser.parse("**hello** world");
//! let ast = Ast::new(parsed);
//!
//! // Iterate over typed blocks
//! for block in ast.blocks() {
//!     if let Some(para) = Paragraph::cast(block) {
//!         // Process paragraph
//!     }
//! }
//! ```

use crate::parser::{Parsed, SyntaxKind, SyntaxNode};

pub use rowan::ast::AstNode;

/// A reference to a span of text. Indexes are in bytes. Start is inclusive, end is not.
/// We use u32 for wasm compatibility.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl From<rowan::TextRange> for Span {
    fn from(range: rowan::TextRange) -> Self {
        Self {
            start: range.start().into(),
            end: range.end().into(),
        }
    }
}

// ============ Block Elements ============

/// A markdown document (root node).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Document(SyntaxNode);

impl AstNode for Document {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Document
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Document {
    /// Iterate over all block-level children.
    pub fn blocks(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0.children().filter(|node| {
            matches!(
                node.kind(),
                SyntaxKind::Paragraph
                    | SyntaxKind::Header
                    | SyntaxKind::List
                    | SyntaxKind::BlockQuote
                    | SyntaxKind::CodeBlock
            )
        })
    }
}

/// A paragraph block.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Paragraph(SyntaxNode);

impl AstNode for Paragraph {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Paragraph
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Paragraph {
    /// Get the text content of the paragraph.
    pub fn text(&self) -> String {
        self.0.text().to_string()
    }

    /// Iterate over inline children.
    pub fn inlines(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0.children()
    }
}

/// A header block with a marker (#, ##, etc.) and text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Header(SyntaxNode);

impl AstNode for Header {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Header
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Header {
    /// Get the header level (1-6).
    pub fn level(&self) -> u8 {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::HeaderMarker)
            .map(|n| n.text().to_string().len() as u8)
            .unwrap_or(1)
    }

    /// Get the header text content.
    pub fn text(&self) -> String {
        let text = self.0.text().to_string();
        text.trim_start_matches('#').trim().to_string()
    }
}

/// A list block containing list items.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct List(SyntaxNode);

impl AstNode for List {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::List
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl List {
    /// Iterate over list items.
    pub fn items(&self) -> impl Iterator<Item = ListItem> + '_ {
        self.0.children().filter_map(|node| ListItem::cast(node))
    }

    /// Check if this is a numbered list.
    pub fn is_numbered(&self) -> bool {
        self.items().next().map_or(false, |item| item.is_numbered())
    }
}

/// A list item.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListItem(SyntaxNode);

impl AstNode for ListItem {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ListItem
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ListItem {
    /// Get the text content of the item (excluding marker).
    pub fn text(&self) -> String {
        self.0.text().to_string()
    }

    /// Check if this item is numbered.
    pub fn is_numbered(&self) -> bool {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::ListMarker)
            .map_or(false, |marker| marker.text().to_string().contains('.'))
    }
}

/// A blockquote.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockQuote(SyntaxNode);

impl AstNode for BlockQuote {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BlockQuote
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl BlockQuote {
    /// Get the quote text content.
    pub fn text(&self) -> String {
        let text = self.0.text().to_string();
        text.trim_start_matches('>').trim().to_string()
    }
}

/// A fenced code block.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodeBlock(SyntaxNode);

impl AstNode for CodeBlock {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CodeBlock
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CodeBlock {
    /// Get the code content (without fences).
    pub fn code(&self) -> String {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::CodeBlockContent)
            .map(|n| n.text().to_string())
            .unwrap_or_default()
    }
}

// ============ Inline Elements ============

/// Strong (bold) text: **text**
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Strong(SyntaxNode);

impl AstNode for Strong {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Strong
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Strong {
    /// Get the text content inside the strong delimiters.
    pub fn text(&self) -> String {
        let text = self.0.text().to_string();
        text.trim_start_matches("**")
            .trim_end_matches("**")
            .to_string()
    }
}

/// Emphasized (italic) text: *text*
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Emphasis(SyntaxNode);

impl AstNode for Emphasis {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Emphasis
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Emphasis {
    /// Get the text content inside the emphasis delimiters.
    pub fn text(&self) -> String {
        let text = self.0.text().to_string();
        text.trim_start_matches('*')
            .trim_end_matches('*')
            .to_string()
    }
}

/// Strikethrough text: ~~text~~
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Strikethrough(SyntaxNode);

impl AstNode for Strikethrough {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Strikethrough
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Strikethrough {
    /// Get the text content inside the strikethrough delimiters.
    pub fn text(&self) -> String {
        let text = self.0.text().to_string();
        text.trim_start_matches("~~")
            .trim_end_matches("~~")
            .to_string()
    }
}

/// Inline code: `code`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InlineCode(SyntaxNode);

impl AstNode for InlineCode {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::InlineCode
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl InlineCode {
    /// Get the code content (without backticks).
    pub fn code(&self) -> String {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::InlineCodeContent)
            .map(|n| n.text().to_string())
            .unwrap_or_default()
    }
}

/// A markdown link: [text](url)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link(SyntaxNode);

impl AstNode for Link {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Link
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Link {
    /// Get the link text.
    pub fn text(&self) -> String {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::LinkText)
            .map(|n| {
                let text = n.text().to_string();
                text.trim_matches(|c| c == '[' || c == ']').to_string()
            })
            .unwrap_or_default()
    }

    /// Get the link destination URL.
    pub fn destination(&self) -> String {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::LinkDestination)
            .map(|n| {
                let text = n.text().to_string();
                text.trim_matches(|c| c == '(' || c == ')').to_string()
            })
            .unwrap_or_default()
    }
}

/// An autolink: <url> or bare URL
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Autolink(SyntaxNode);

impl AstNode for Autolink {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Autolink
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Autolink {
    /// Get the URL.
    pub fn url(&self) -> String {
        self.0.text().to_string()
    }
}

/// An angle bracket link: <https://example.com>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AngleBracketLink(SyntaxNode);

impl AstNode for AngleBracketLink {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::AngleBracketLink
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl AngleBracketLink {
    /// Get the URL.
    pub fn url(&self) -> String {
        let text = self.0.text().to_string();
        text.trim_matches(|c| c == '<' || c == '>').to_string()
    }
}

// ============ Special Elements ============

/// A user mention: @uuid or <@uuid>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mention(SyntaxNode);

impl AstNode for Mention {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Mention
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Mention {
    /// Get the mention UUID.
    pub fn uuid(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(|n| n.into_token())
            .find(|t| t.kind() != SyntaxKind::MentionMarker)
            .map(|t| t.text().to_string())
            .unwrap_or_default()
    }
}

/// An emoji: <:name:uuid> or <a:name:uuid>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Emoji(SyntaxNode);

impl AstNode for Emoji {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Emoji
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Emoji {
    /// Get the emoji name.
    pub fn name(&self) -> String {
        self.0
            .children()
            .find(|n| n.kind() == SyntaxKind::EmojiName)
            .map(|n| n.text().to_string())
            .unwrap_or_default()
    }

    /// Get the emoji UUID.
    pub fn uuid(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(|n| n.into_token())
            .find(|t| {
                let text = t.text();
                // UUID pattern: 8-4-4-4-12 hex chars
                text.len() == 36 && text.chars().filter(|c| *c == '-').count() == 4
            })
            .map(|t| t.text().to_string())
            .unwrap_or_default()
    }
}

/// An escape sequence: \char
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Escape(SyntaxNode);

impl AstNode for Escape {
    type Language = crate::parser::MyLang;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Escape
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Escape {
    /// Get the escaped character.
    pub fn escaped_char(&self) -> Option<char> {
        self.0
            .children_with_tokens()
            .find_map(|n| n.into_token())
            .and_then(|t| t.text().chars().next())
    }
}

// ============ High-level Ast wrapper ============

/// A parsed markdown document.
///
/// Wraps the syntax tree and original source text, providing convenient access methods
/// and typed iteration over blocks.
///
/// # Example
/// ```
/// use lamprey_markdown::{Parser, Ast};
/// use lamprey_markdown::ast::{AstNode, Paragraph, Strong};
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
///
/// // Iterate over typed blocks
/// for block in ast.blocks() {
///     if let Some(para) = Paragraph::cast(block) {
///         println!("Paragraph: {}", para.text());
///     }
/// }
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

    /// Get the document node.
    pub fn document(&self) -> Option<Document> {
        self.syntax()
            .children()
            .find_map(|node| Document::cast(node))
    }

    /// Iterate over all block-level elements in the document.
    pub fn blocks(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.document()
            .map(|d| d.blocks().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    /// Find all paragraphs in the document.
    pub fn paragraphs(&self) -> impl Iterator<Item = Paragraph> + '_ {
        self.blocks().filter_map(|node| Paragraph::cast(node))
    }

    /// Find all headers in the document.
    pub fn headers(&self) -> impl Iterator<Item = Header> + '_ {
        self.blocks().filter_map(|node| Header::cast(node))
    }

    /// Find all lists in the document.
    pub fn lists(&self) -> impl Iterator<Item = List> + '_ {
        self.blocks().filter_map(|node| List::cast(node))
    }

    /// Find all blockquotes in the document.
    pub fn blockquotes(&self) -> impl Iterator<Item = BlockQuote> + '_ {
        self.blocks().filter_map(|node| BlockQuote::cast(node))
    }

    /// Find all code blocks in the document.
    pub fn code_blocks(&self) -> impl Iterator<Item = CodeBlock> + '_ {
        self.blocks().filter_map(|node| CodeBlock::cast(node))
    }

    /// Find all strong (bold) elements in the document.
    pub fn strong_elements(&self) -> impl Iterator<Item = Strong> + '_ {
        self.syntax()
            .descendants()
            .filter_map(|node| Strong::cast(node))
    }

    /// Find all emphasis (italic) elements in the document.
    pub fn emphasis_elements(&self) -> impl Iterator<Item = Emphasis> + '_ {
        self.syntax()
            .descendants()
            .filter_map(|node| Emphasis::cast(node))
    }

    /// Find all links in the document.
    pub fn links(&self) -> impl Iterator<Item = Link> + '_ {
        self.syntax()
            .descendants()
            .filter_map(|node| Link::cast(node))
    }

    /// Find all mentions in the document.
    pub fn mentions(&self) -> impl Iterator<Item = Mention> + '_ {
        self.syntax()
            .descendants()
            .filter_map(|node| Mention::cast(node))
    }

    /// Find all emojis in the document.
    pub fn emojis(&self) -> impl Iterator<Item = Emoji> + '_ {
        self.syntax()
            .descendants()
            .filter_map(|node| Emoji::cast(node))
    }
}
