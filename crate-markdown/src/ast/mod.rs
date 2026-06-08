use crate::prelude::*;

pub mod block;
pub mod inline;
pub mod table;
pub mod transform;

/// abstract syntax tree
///
/// this is a strongly typed node that corresponds to an underlying parser node
pub trait AstNode: Sized {
    /// check if a node can be convert into this
    fn can_cast(node: &Node) -> bool;

    /// attempt to convert a node into this
    fn cast(tn: TreeNode) -> Result<Self, TreeNode>;

    /// get the underlying tree node
    fn node(&self) -> &TreeNode;

    fn cast_raw(tree: Ref<Tree>, node: Node) -> Result<Self, Node> {
        Self::cast(TreeNode { tree, node }).map_err(|e| e.node)
    }

    fn node_raw(&self) -> &Node {
        &self.node().node
    }
}

/// helper to make tree + node pairs easier to work with
pub struct TreeNode {
    pub(crate) tree: Ref<Tree>,
    pub(crate) node: Node,
}

impl TreeNode {
    /// get the text of this node
    pub fn text(&self) -> &str {
        let span = self.node.span();
        &self.tree.source()[span.start as usize..span.end as usize]
    }

    /// get the children of this node
    pub fn children(&self) -> impl Iterator<Item = TreeNode> + '_ {
        self.node.children.iter().map(|n| TreeNode {
            tree: Ref::clone(&self.tree),
            node: self.tree[*n].clone(),
        })
    }
}

macro_rules! impl_ast {
    ($name:ident, $kind:pat $(if $guard:expr)?) => {
        impl $crate::ast::AstNode for $name {
            fn can_cast(node: &$crate::tree::node::Node) -> bool {
                matches!(node.kind(), $kind $(if $guard)?)
            }

            fn cast(tn: $crate::ast::TreeNode) -> Result<Self, $crate::ast::TreeNode> {
                if Self::can_cast(&tn.node) {
                    Ok(Self(tn))
                } else {
                    Err(tn)
                }
            }

            fn node(&self) -> &$crate::ast::TreeNode {
                &self.0
            }
        }
    };
}

pub(crate) use impl_ast;
