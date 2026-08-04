use crate::ast::impl_ast;
use crate::ast::inline::Inline;
use crate::ast::list::List;
use crate::prelude::*;
use crate::tree::node::MarkdownLanguage;

/// the top level document
#[derive(Debug)]
pub struct Document(SyntaxNode);
#[derive(Debug)]
pub struct Paragraph(SyntaxNode);
#[derive(Debug)]
pub struct Blockquote(SyntaxNode);
#[derive(Debug)]
pub struct Codeblock(SyntaxNode);
#[derive(Debug)]
pub struct Header(SyntaxNode);

pub use crate::ast::table::Table;

impl_ast!(Document, NodeKind::Document);
impl_ast!(Paragraph, NodeKind::Block(BlockKind::Paragraph));
impl_ast!(Blockquote, NodeKind::Block(BlockKind::Blockquote));
impl_ast!(Codeblock, NodeKind::Block(BlockKind::Codeblock));
impl_ast!(Header, NodeKind::Block(b) if b.is_header());

/// any block type node
#[derive(Debug)]
pub enum Block {
    Header(Header),
    Paragraph(Paragraph),
    Blockquote(Blockquote),
    Codeblock(Codeblock),
    List(List),
    Table(Table),
}

impl AstNode for Block {
    type Language = MarkdownLanguage;

    fn can_cast(kind: NodeKind) -> bool {
        kind.is_block() && kind != NodeKind::Document
    }

    fn cast(tn: SyntaxNode) -> Option<Self> {
        let kind = tn.kind();
        if Header::can_cast(kind) {
            Header::cast(tn).map(Self::Header)
        } else if Paragraph::can_cast(kind) {
            Paragraph::cast(tn).map(Self::Paragraph)
        } else if Blockquote::can_cast(kind) {
            Blockquote::cast(tn).map(Self::Blockquote)
        } else if Codeblock::can_cast(kind) {
            Codeblock::cast(tn).map(Self::Codeblock)
        } else if List::can_cast(kind) {
            List::cast(tn).map(Self::List)
        } else if Table::can_cast(kind) {
            Table::cast(tn).map(Self::Table)
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Block::Header(b) => b.syntax(),
            Block::Paragraph(b) => b.syntax(),
            Block::Blockquote(b) => b.syntax(),
            Block::Codeblock(b) => b.syntax(),
            Block::List(b) => b.syntax(),
            Block::Table(b) => b.syntax(),
        }
    }
}

impl Header {
    pub fn level(&self) -> u8 {
        self.0
            .children_with_tokens()
            .find_map(|child| {
                if child.kind() == NodeKind::Text(TextKind::HeaderHashes) {
                    // NOTE: does this include the space between the hashes and content?
                    // PERF: consider using .text_range() then end - start instead
                    Some(child.to_string().len() as u8)
                } else {
                    None
                }
            })
            .unwrap_or(1)
    }

    pub fn children<'a>(&'a self) -> impl Iterator<Item = Inline> + 'a {
        self.0
            .children_with_tokens()
            .filter_map(|child| match child.kind() {
                NodeKind::Text(TextKind::HeaderHashes) | NodeKind::Text(TextKind::Padding) => None,
                _ => Inline::cast(child),
            })
    }
}

impl Codeblock {
    /// get the language tag for this code block
    pub fn language(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .find(|c| c.kind() == NodeKind::Text(TextKind::CodeblockLang))
            .map(|c| c.to_string())
    }

    /// iterate over the content of this code block
    pub fn content<'a>(&'a self) -> impl Iterator<Item = Inline> + 'a {
        self.0
            .children_with_tokens()
            .filter_map(|child| match child.kind() {
                NodeKind::Text(TextKind::Syntax)
                | NodeKind::Text(TextKind::CodeblockLang)
                | NodeKind::Text(TextKind::Padding) => None,
                _ => Inline::cast(child),
            })
    }
}

impl Document {
    pub fn children<'a>(&'a self) -> impl Iterator<Item = Block> + 'a {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(Block::cast))
    }
}

impl Paragraph {
    pub fn children<'a>(&'a self) -> impl Iterator<Item = Inline> + 'a {
        self.0
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }
}

impl Blockquote {
    pub fn children<'a>(&'a self) -> impl Iterator<Item = Block> + 'a {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(Block::cast))
    }
}
