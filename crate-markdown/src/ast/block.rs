use crate::ast::impl_ast;
use crate::ast::inline::Inline;
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
pub struct List(SyntaxNode);
#[derive(Debug)]
pub struct ListItem(SyntaxNode);
#[derive(Debug)]
pub struct Header(SyntaxNode);

impl_ast!(Document, NodeKind::Document);
impl_ast!(Paragraph, NodeKind::Block(BlockKind::Paragraph));
impl_ast!(Blockquote, NodeKind::Block(BlockKind::Blockquote));
impl_ast!(Codeblock, NodeKind::Block(BlockKind::Codeblock));
impl_ast!(ListItem, NodeKind::Block(BlockKind::ListItem));
impl_ast!(
    List,
    NodeKind::Block(BlockKind::ListOrdered)
        | NodeKind::Block(BlockKind::ListUnordered)
        | NodeKind::Block(BlockKind::ListTasks)
);
impl_ast!(Header, NodeKind::Block(b) if b.is_header());

/// any block type node
#[derive(Debug)]
pub enum Block {
    Header(Header),
    Paragraph(Paragraph),
    Blockquote(Blockquote),
    Codeblock(Codeblock),
    List(List),
    ListItem(ListItem),
}

pub enum ListKind {
    Ordered,
    Unordered,
    Task,
}

impl List {
    pub fn kind(&self) -> ListKind {
        match self.0.kind() {
            NodeKind::Block(BlockKind::ListOrdered) => ListKind::Ordered,
            NodeKind::Block(BlockKind::ListTasks) => ListKind::Task,
            _ => ListKind::Unordered,
        }
    }

    pub fn items(&self) -> impl Iterator<Item = ListItem> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(ListItem::cast))
    }
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
        } else if ListItem::can_cast(kind) {
            ListItem::cast(tn).map(Self::ListItem)
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
            Block::ListItem(b) => b.syntax(),
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
        self.0.children_with_tokens().filter_map(|child| {
            if child.kind() == NodeKind::Text(TextKind::HeaderHashes) {
                None
            } else {
                Inline::cast(child)
            }
        })
    }
}

impl Codeblock {
    pub fn language(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .find(|c| c.kind() == NodeKind::Text(TextKind::CodeblockLang))
            .map(|c| c.to_string())
    }
}

impl ListItem {
    pub fn content(&self) -> impl Iterator<Item = Block> + '_ {
        // TODO: filter out ListPrefix syntax
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(Block::cast))
    }

    pub fn number(&self) -> Option<u64> {
        self.0
            .children_with_tokens()
            .find(|c| c.kind() == NodeKind::Text(TextKind::ListPrefix))
            // NOTE: do i want to use the user defined number or automatically increment? i *think* commonmark always autoincrements starting from the first list item's number.
            .and_then(|c| c.to_string().trim_end_matches('.').parse().ok())
    }

    pub fn completed(&self) -> Option<bool> {
        self.0
            .children_with_tokens()
            .find(|c| c.kind() == NodeKind::Text(TextKind::TaskCheck))
            .map(|c| c.to_string().contains('x'))
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
