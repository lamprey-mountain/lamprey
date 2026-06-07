use crate::{prelude::*, tree::{Node, Tree}};

/// a cursor for traversing a tree
pub struct TreeCursor<'tree> {
    tree: &'tree Tree,
    path: Box<[u16]>,
    // path: Ref<[u16]>,
    current: Ref<Node>,
    // openStart: boolean

    //     Whether the start of the fragment represents the start of a parse, or the end of a change. (In the second case, it may not be safe to reuse some nodes at the start, depending on the parsing algorithm.)
    // openEnd: boolean

    //     Whether the end of the fragment represents the end of a full-document parse, or the start of a change.
}

impl<'tree> TreeCursor<'tree> {
    /// go to the next sibling node
    pub fn next(&mut self) -> Option<&Node> {
        todo!()
    }

    /// go to the previous sibling node
    pub fn prev(&mut self) -> Option<&Node> {
        todo!()
    }

    /// go to the parent node
    pub fn parent(&mut self) -> Option<&Node> {
        todo!()
    }

    /// go to the root node
    pub fn root(&mut self) -> &Node {
        todo!()
    }

    /// go to a specific offset in the tree
    pub fn goto(&mut self, position: Len) -> Option<&Node> {
        todo!()
    }

    /// get the current node
    pub fn node(&self) -> &Node {
        todo!()
    }

    // /// iterate over children nodes
    // pub fn children(&'tree self) -> impl Iterator<Item = &'tree TreeCursor> {
    //     todo!()
    // }
}
