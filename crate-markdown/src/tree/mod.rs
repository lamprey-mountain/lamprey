//! syntax tree types

use crate::prelude::*;

pub mod kind;
pub mod node;

/// an immutable parsed syntax tree
// TODO: remove? use rowan types directly?
pub struct Tree {
    /// the root green node
    pub(crate) root: GreenNode,
}

impl Tree {
    /// get the root node
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }
}
