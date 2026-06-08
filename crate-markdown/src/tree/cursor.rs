use crate::prelude::*;

/// a cursor for traversing a tree
#[derive(Clone)]
pub struct TreeCursor<'tree> {
    tree: &'tree Tree,
    /// path from root to the current node
    ///
    /// stores (parent_node_index, current_node_index_in_parent)
    ///
    /// current_node_index_in_parent is undefined for the root node (first item in path)
    path: Vec<(SyntaxIndex, usize)>,
}

impl<'tree> TreeCursor<'tree> {
    pub(crate) fn new(tree: &'tree Tree) -> Self {
        Self {
            tree,
            path: vec![(SyntaxIndex(0), 0)],
        }
    }

    /// get the current node
    pub fn node(&self) -> &'tree SyntaxData {
        let (parent, idx) = self.path.last().unwrap();
        if parent.0 == 0 && self.path.len() == 1 {
            &self.tree[*parent]
        } else {
            &self.tree[self.tree[*parent].children[*idx]]
        }
    }

    /// go to the root node
    pub fn root(&mut self) -> &'tree SyntaxData {
        self.path.clear();
        self.path.push((SyntaxIndex(0), 0));
        self.node()
    }

    /// go to the parent node
    pub fn parent(&mut self) -> Option<&'tree SyntaxData> {
        if self.path.len() > 1 {
            self.path.pop();
            Some(self.node())
        } else {
            None
        }
    }

    /// go to the next sibling node
    pub fn next(&mut self) -> Option<&'tree SyntaxData> {
        let (parent, idx) = self.path.last_mut()?;
        let children = &self.tree[*parent].children;
        if *idx + 1 < children.len() {
            *idx += 1;
            Some(self.node())
        } else {
            None
        }
    }

    /// go to the previous sibling node
    pub fn prev(&mut self) -> Option<&'tree SyntaxData> {
        let (_parent, idx) = self.path.last_mut()?;
        if *idx > 0 {
            *idx -= 1;
            Some(self.node())
        } else {
            None
        }
    }

    /// get the zero-based index of the current node among its siblings
    pub fn index(&self) -> Option<usize> {
        self.path.last().map(|(_, idx)| *idx)
    }

    /// get the depth of the current node in the tree
    pub fn depth(&self) -> Option<usize> {
        Some(self.path.len() - 1)
    }
}
