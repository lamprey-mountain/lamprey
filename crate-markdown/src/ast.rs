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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Get the underlying syntax node.
    pub fn syntax_node(&self) -> &SyntaxNode {
        &self.0
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

    /// Find all emojis in the document.
    pub fn emojis(&self) -> impl Iterator<Item = Emoji> + '_ {
        self.syntax()
            .descendants()
            .filter_map(|node| Emoji::cast(node))
    }
}

// ============ AST Query Helpers ============

/// A mention ID that can be extracted from markdown.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MentionId {
    User(uuid::Uuid),
    Role(uuid::Uuid),
    Channel(uuid::Uuid),
    Emoji {
        id: uuid::Uuid,
        name: String,
        animated: bool,
    },
    Everyone,
}

/// Collection of mention IDs extracted from markdown.
#[derive(Debug, Default, Clone)]
pub struct MentionIds {
    pub users: Vec<uuid::Uuid>,
    pub roles: Vec<uuid::Uuid>,
    pub channels: Vec<uuid::Uuid>,
    pub emojis: Vec<(uuid::Uuid, String, bool)>, // (id, name, animated)
    pub everyone: bool,
}

impl FromIterator<MentionId> for MentionIds {
    fn from_iter<I: IntoIterator<Item = MentionId>>(iter: I) -> Self {
        let mut result = MentionIds::default();
        for mention in iter {
            match mention {
                MentionId::User(id) => result.users.push(id),
                MentionId::Role(id) => result.roles.push(id),
                MentionId::Channel(id) => result.channels.push(id),
                MentionId::Emoji { id, name, animated } => result.emojis.push((id, name, animated)),
                MentionId::Everyone => result.everyone = true,
            }
        }
        result
    }
}

/// A link extracted from markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRef<'a> {
    /// The link destination (URL).
    pub dest: std::borrow::Cow<'a, str>,
    /// The link text (if any).
    pub text: Option<std::borrow::Cow<'a, str>>,
    /// The type of link.
    pub kind: LinkKind,
}

/// The type of link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// Raw URL (autolink): https://example.com
    RawUrl,
    /// Angle bracket link: <https://example.com>
    AngleBracket,
    /// Named link: [text](url)
    Named,
}

/// Iterator over links in the AST.
pub struct LinksIter<'a> {
    nodes: std::iter::Peekable<
        std::boxed::Box<dyn Iterator<Item = rowan::SyntaxNode<crate::parser::MyLang>> + 'a>,
    >,
    source: &'a str,
}

impl<'a> Iterator for LinksIter<'a> {
    type Item = LinkRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use crate::parser::SyntaxKind;

        while let Some(node) = self.nodes.next() {
            match node.kind() {
                SyntaxKind::Autolink => {
                    let dest = node.text().to_string();
                    return Some(LinkRef {
                        dest: std::borrow::Cow::Owned(dest),
                        text: None,
                        kind: LinkKind::RawUrl,
                    });
                }
                SyntaxKind::AngleBracketLink => {
                    let text = node.text().to_string();
                    // Strip < and >
                    let dest = text.trim_matches(|c| c == '<' || c == '>');
                    return Some(LinkRef {
                        dest: std::borrow::Cow::Owned(dest.to_string()),
                        text: None,
                        kind: LinkKind::AngleBracket,
                    });
                }
                SyntaxKind::Link => {
                    let mut dest = None;
                    let mut text = None;

                    for child in node.children() {
                        match child.kind() {
                            SyntaxKind::LinkText => {
                                let t = child.text().to_string();
                                // Strip [ and ]
                                let trimmed = t.trim_matches(|c| c == '[' || c == ']');
                                text = Some(std::borrow::Cow::Owned(trimmed.to_string()));
                            }
                            SyntaxKind::LinkDestination => {
                                let t = child.text().to_string();
                                // Strip ( and )
                                let trimmed = t.trim_matches(|c| c == '(' || c == ')');
                                dest = Some(std::borrow::Cow::Owned(trimmed.to_string()));
                            }
                            _ => {}
                        }
                    }

                    if let Some(dest) = dest {
                        return Some(LinkRef {
                            dest,
                            text,
                            kind: LinkKind::Named,
                        });
                    }
                }
                _ => {}
            }
        }

        None
    }
}

/// Iterator over mention IDs in the AST.
pub struct MentionsIter<'a> {
    nodes: std::iter::Peekable<
        std::boxed::Box<dyn Iterator<Item = rowan::SyntaxNode<crate::parser::MyLang>> + 'a>,
    >,
    source: &'a str,
}

impl<'a> Iterator for MentionsIter<'a> {
    type Item = MentionId;

    fn next(&mut self) -> Option<Self::Item> {
        use crate::parser::SyntaxKind;

        while let Some(node) = self.nodes.next() {
            match node.kind() {
                SyntaxKind::Mention => {
                    // Extract UUID from mention
                    let mut uuid_str = String::new();
                    for child in node.children_with_tokens() {
                        if let rowan::NodeOrToken::Token(token) = child {
                            if token.kind() != SyntaxKind::MentionMarker {
                                uuid_str.push_str(token.text());
                            }
                        }
                    }

                    if let Ok(uuid) = uuid::Uuid::parse_str(&uuid_str) {
                        return Some(MentionId::User(uuid));
                    }
                }
                SyntaxKind::Emoji => {
                    let mut name = String::new();
                    let mut uuid_str = String::new();
                    let mut animated = false;

                    for child in node.children_with_tokens() {
                        match child {
                            rowan::NodeOrToken::Node(child_node) => {
                                if child_node.kind() == SyntaxKind::EmojiName {
                                    name = child_node.text().to_string();
                                }
                            }
                            rowan::NodeOrToken::Token(token) => {
                                let text = token.text();
                                // Check for animated marker "a"
                                if token.kind() == SyntaxKind::EmojiMarker && text == "a" {
                                    animated = true;
                                }
                                // Get UUID
                                if text.len() == 36
                                    && text.chars().filter(|c| *c == '-').count() == 4
                                {
                                    uuid_str = text.to_string();
                                }
                            }
                        }
                    }

                    if let Ok(uuid) = uuid::Uuid::parse_str(&uuid_str) {
                        return Some(MentionId::Emoji {
                            id: uuid,
                            name,
                            animated,
                        });
                    }
                }
                SyntaxKind::Paragraph => {
                    // Check for @everyone in paragraph text
                    let text = node.text().to_string();
                    if text.contains("@everyone") {
                        return Some(MentionId::Everyone);
                    }
                }
                _ => {}
            }
        }

        None
    }
}

impl Ast {
    /// Iterate over all links in the document.
    ///
    /// This includes:
    /// - Raw URLs (autolinks): https://example.com
    /// - Angle bracket links: <https://example.com>
    /// - Named links: [text](url)
    ///
    /// # Example
    /// ```
    /// use lamprey_markdown::{Parser, Ast};
    ///
    /// let parser = Parser::default();
    /// let ast = Ast::new(parser.parse("check [example](https://example.com) and https://other.com"));
    ///
    /// let links: Vec<_> = ast.links().collect();
    /// assert_eq!(links.len(), 2);
    /// ```
    pub fn links(&self) -> impl Iterator<Item = LinkRef<'_>> + '_ {
        let iter: Box<dyn Iterator<Item = _> + '_> = Box::new(self.syntax().descendants());
        LinksIter {
            nodes: iter.peekable(),
            source: self.source(),
        }
    }

    /// Iterate over all mention IDs in the document.
    ///
    /// This extracts:
    /// - User mentions: <@uuid>
    /// - Emoji mentions: <:name:uuid> or <a:name:uuid>
    /// - @everyone mentions
    ///
    /// # Example
    /// ```
    /// use lamprey_markdown::{Parser, Ast};
    /// use lamprey_markdown::ast::{MentionId, MentionIds};
    ///
    /// let parser = Parser::default();
    /// let ast = Ast::new(parser.parse("hello <@uuid> and <:emoji:uuid>"));
    ///
    /// let mentions: MentionIds = ast.mentions().collect();
    /// assert!(!mentions.users.is_empty());
    /// assert!(!mentions.emojis.is_empty());
    /// ```
    pub fn mentions(&self) -> impl Iterator<Item = MentionId> + '_ {
        let iter: Box<dyn Iterator<Item = _> + '_> = Box::new(self.syntax().descendants());
        MentionsIter {
            nodes: iter.peekable(),
            source: self.source(),
        }
    }
}
