//! syntax tree types

use crate::prelude::*;
use crate::tree::cursor::TreeCursor;

pub mod cursor;
pub mod kind;
pub mod node;

/// an immutable parsed syntax tree
// TODO: impl Debug?
pub struct Tree {
    /// parsed nodes in this tree
    ///
    /// the root node is at index 0
    node: Vec<SyntaxData>,

    /// the source string
    // NOTE: i could make nodes store source string fragments instead?
    source: String,
}

pub struct TreeBuilder {
    nodes: Vec<SyntaxData>,
    source: String,
    // NOTE: i could deduplicate nodes
    // cache: HashMap<Node, NodeIndex>,
}

/// incremental parsing cache
pub struct Cache<'a> {
    old_tree: &'a Tree,

    /// the span that was replaced with new text
    edit_span: Span,

    /// how many chars were added/removed
    delta: isize,
    // rowan:
    // nodes: HashMap<NoHash<GreenNode>, ()>,
    // tokens: HashMap<NoHash<GreenToken>, ()>,
}

impl Tree {
    /// create an empty tree
    // TODO: remove?
    pub(crate) fn empty(source: String) -> Self {
        Self {
            node: vec![SyntaxData {
                kind: NodeKind::Document,
                span: (0, source.len() as Len).into(),
                children: vec![],
            }],
            source,
        }
    }

    /// create a cursor for traversing this tree
    pub fn cursor(&self) -> TreeCursor<'_> {
        TreeCursor::new(self)
    }

    /// get the markdown source text
    pub fn source(&self) -> &str {
        &self.source
    }

    /// get the root node
    pub fn root(&self) -> &SyntaxData {
        &self.node[0]
    }
}

impl std::ops::Index<SyntaxIndex> for Tree {
    type Output = SyntaxData;

    fn index(&self, index: SyntaxIndex) -> &Self::Output {
        &self.node[index.0 as usize]
    }
}

impl TreeBuilder {
    pub fn new(source: String) -> Self {
        Self {
            nodes: Vec::new(),
            source,
        }
    }

    pub fn push_node(&mut self, kind: NodeKind, span: Span) -> SyntaxIndex {
        let index = SyntaxIndex(self.nodes.len() as u32);
        self.nodes.push(SyntaxData {
            kind,
            span,
            children: vec![],
        });
        index
    }

    pub fn add_child(&mut self, parent: SyntaxIndex, child: SyntaxIndex) {
        self.nodes[parent.0 as usize].children.push(child);
    }

    pub fn build(self) -> Tree {
        Tree {
            node: self.nodes,
            source: self.source,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

impl<'a> Cache<'a> {
    pub fn new(old_tree: &'a Tree, edit_span: Span, delta: isize) -> Self {
        Self {
            old_tree,
            edit_span,
            delta,
        }
    }

    /// checks if a block from the old tree can be reused at the given byte offset
    pub fn find_reusable_block(&self, pos: Len) -> Option<SyntaxIndex> {
        // Find a top-level block node in the old tree that starts at `pos` (before delta if pos > edit)
        // Ensure its span does NOT overlap with `edit_span`.
        // For simplicity in this mock incremental parser, we return None to force re-parse.
        // A full implementation would find matching unchanged nodes.
        todo!()
    }
}

// impl Cache {
//     pub fn new() -> Self {}

//     pub fn insert(&mut self, node: ()) -> SyntaxData {}
// }
