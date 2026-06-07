//! tree types

use crate::prelude::*;
use crate::tree::cursor::TreeCursor;
use crate::tree::node::{Node, NodeIndex};

pub mod cursor;
pub mod node;

/// an immutable parsed syntax tree
// TODO: impl Debug?
pub struct Tree {
    /// parsed nodes in this tree
    ///
    /// the root node is at index 0
    node: Vec<Node>,

    /// the source string
    // NOTE: i could make nodes store source string fragments instead?
    source: String,
}

pub struct TreeBuilder {
    tree: Tree,
    // cache: HashMap<Node, NodeIndex>,
}

impl Tree {
    // fn empty() -> Self {
    //     todo!()
    // }

    /// create a cursor for traversing this tree
    pub fn cursor(&self) -> TreeCursor<'_> {
        TreeCursor::new(self)
    }
}

impl std::ops::Index<NodeIndex> for Tree {
    type Output = Node;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.node[index.0 as usize]
    }
}

impl TreeBuilder {
    pub(crate) fn build(self) -> Tree {
        todo!()
    }
}
