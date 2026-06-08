use crate::ast::impl_ast;
use crate::prelude::*;

/// the top level document
pub struct Document(SyntaxNode);
pub struct Paragraph(SyntaxNode);
pub struct Blockquote(SyntaxNode);
pub struct Codeblock(SyntaxNode);
pub struct List(SyntaxNode);
pub struct ListItem(SyntaxNode);
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
pub enum Block {
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
        match self.0.kind() {
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
    fn can_cast(tn: &SyntaxNode) -> bool {
        tn.kind().is_block() && tn.kind() != NodeKind::Document
    }

    fn cast(tn: SyntaxNode) -> Result<Self, SyntaxNode> {
        if Header::can_cast(&tn) {
            Ok(Self::Header(Header(tn)))
        } else if Paragraph::can_cast(&tn) {
            Ok(Self::Paragraph(Paragraph(tn)))
        } else if Blockquote::can_cast(&tn) {
            Ok(Self::Blockquote(Blockquote(tn)))
        } else if Codeblock::can_cast(&tn) {
            Ok(Self::Codeblock(Codeblock(tn)))
        } else if ListItem::can_cast(&tn) {
            Ok(Self::ListItem(ListItem(tn)))
        } else {
            Err(tn)
        }
    }

    fn node(&self) -> &SyntaxNode {
        match self {
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
                if child.kind() == NodeKind::Text(TextKind::HeaderHashes) {
                    // NOTE: does this include the space between the hashes and content?
                    Some(u32::from(child.text().len()) as u8)
                } else {
                    None
                }
            })
            .unwrap_or(1)
    }

    // TODO: make iterator Item = Inline
    pub fn children<'a>(&'a self) -> impl Iterator<Item = Block> + 'a {
        self.0.children().filter_map(|child| {
            if child.kind() == NodeKind::Text(TextKind::HeaderHashes) {
                None
            } else {
                Block::cast(child).ok()
            }
        })
    }
}

impl Codeblock {
    pub fn language(&self) -> Option<String> {
        self.0
            .children()
            .find(|c| c.kind() == NodeKind::Text(TextKind::CodeblockLang))
            .map(|c| c.text().to_string())
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
            .find(|c| c.kind() == NodeKind::Text(TextKind::ListPrefix))
            // NOTE: do i want to use the user defined number or automatically increment? i *think* commonmark always autoincrements starting from the first list item's number.
            .and_then(|c| c.text().to_string().trim_end_matches('.').parse().ok())
    }

    pub fn completed(&self) -> Option<bool> {
        self.0
            .children()
            .find(|c| c.kind() == NodeKind::Text(TextKind::TaskCheck))
            .map(|c| c.text().contains_char('x'))
    }
}

// TODO: add children method to Document
// pub fn children<'a>(&'a self) -> impl Iterator<Item = Block> + 'a {

// TODO: add children method to Paragraph
// pub fn children<'a>(&'a self) -> impl Iterator<Item = Inline> + 'a {
