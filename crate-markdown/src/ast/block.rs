use crate::ast::{impl_ast, TreeNode};
use crate::prelude::*;
use crate::tree::node::{BlockKind, NodeKind, TextKind};

/// the top level document
pub struct Document(TreeNode);
pub struct Paragraph(TreeNode);
pub struct Blockquote(TreeNode);
pub struct Codeblock(TreeNode);
pub struct List(TreeNode);
pub struct ListItem(TreeNode);
pub struct Header(TreeNode);

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
pub enum Block {
    Document(Document),
    Header(Header),
    Paragraph(Paragraph),
    Blockquote(Blockquote),
    Codeblock(Codeblock),
    ListItem(ListItem),
}

pub enum ListKind {
    Ordered,
    Unordered,
    Task,
}

impl List {
    pub fn kind(&self) -> ListKind {
        match self.0.node.kind() {
            NodeKind::Block(BlockKind::ListOrdered) => ListKind::Ordered,
            NodeKind::Block(BlockKind::ListTasks) => ListKind::Task,
            _ => ListKind::Unordered,
        }
    }

    pub fn items(&self) -> impl Iterator<Item = ListItem> + '_ {
        self.0
            .children()
            .filter_map(|child| ListItem::cast(child).ok())
    }
}

impl AstNode for Block {
    fn can_cast(node: &Node) -> bool {
        node.kind().is_block() || matches!(node.kind(), NodeKind::Document)
    }

    fn cast(tn: TreeNode) -> Result<Self, TreeNode> {
        if Document::can_cast(&tn.node) {
            Ok(Self::Document(Document(tn)))
        } else if Header::can_cast(&tn.node) {
            Ok(Self::Header(Header(tn)))
        } else if Paragraph::can_cast(&tn.node) {
            Ok(Self::Paragraph(Paragraph(tn)))
        } else if Blockquote::can_cast(&tn.node) {
            Ok(Self::Blockquote(Blockquote(tn)))
        } else if Codeblock::can_cast(&tn.node) {
            Ok(Self::Codeblock(Codeblock(tn)))
        } else if ListItem::can_cast(&tn.node) {
            Ok(Self::ListItem(ListItem(tn)))
        } else {
            Err(tn)
        }
    }

    fn node(&self) -> &TreeNode {
        match self {
            Block::Document(b) => b.node(),
            Block::Header(b) => b.node(),
            Block::Paragraph(b) => b.node(),
            Block::Blockquote(b) => b.node(),
            Block::Codeblock(b) => b.node(),
            Block::ListItem(b) => b.node(),
        }
    }
}

impl Header {
    pub fn level(&self) -> u8 {
        self.0
            .children()
            .find_map(|child| {
                if child.node.kind() == NodeKind::Text(TextKind::HeaderHashes) {
                    child.text().parse().ok()
                } else {
                    None
                }
            })
            .unwrap_or(1)
    }

    pub fn children<'a>(&'a self) -> impl Iterator<Item = Block> + 'a {
        self.0.children().filter_map(|child| {
            // NOTE: do i include the space between the hashes and content?
            if child.node.kind() == NodeKind::Text(TextKind::HeaderHashes) {
                None
            } else {
                Block::cast(child).ok()
            }
        })
    }
}

impl Codeblock {
    pub fn language(&self) -> Option<&str> {
        self.0
            .children()
            .find(|c| c.node.kind() == NodeKind::Text(TextKind::CodeblockLang))
            .map(|c| {
                let span = c.node.span();
                &self.0.tree.source()[span.start as usize..span.end as usize]
            })
    }
}

impl ListItem {
    pub fn content(&self) -> impl Iterator<Item = Block> + '_ {
        // TODO: filter out ListPrefix syntax
        self.0
            .children()
            .filter_map(|child| Block::cast(child).ok())
    }

    pub fn number(&self) -> Option<u64> {
        self.0
            .children()
            .find(|c| c.node.kind() == NodeKind::Text(TextKind::ListPrefix))
            .and_then(|c| c.text().trim_end_matches('.').parse().ok())
    }

    pub fn completed(&self) -> Option<bool> {
        self.0
            .children()
            .find(|c| c.node.kind() == NodeKind::Text(TextKind::TaskCheck))
            .map(|c| c.text().contains('x'))
    }
}
