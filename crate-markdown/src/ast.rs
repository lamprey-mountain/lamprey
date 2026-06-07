use crate::prelude::*;
use crate::tree::node::Node;

/// abstract syntax tree
pub trait Ast: Sized {
    /// attempt to convert a node into this
    fn cast(node: Node) -> Result<Self, Node>;

    /// get the underlying node
    fn node(&self) -> &Node;
}

// TODO: create structs for most node kinds?
// pub struct Document(Node);
// impl Ast for Document { ... }
// impl Document {
//     fn blocks(&self) -> iter over children, map to Ast::cast
// }
