use crate::prelude::*;

pub mod block;
pub mod inline;
pub mod table;
pub mod transform;

/// abstract syntax tree
///
/// this is a strongly typed node that corresponds to an underlying parser node
pub trait AstNode: Sized {
    /// check if a syntax node can be converted into this
    fn can_cast(node: &SyntaxData) -> bool;

    /// attempt to convert a syntax node into this
    fn cast(tn: SyntaxNode) -> Result<Self, SyntaxNode>;

    /// get the underlying syntax node
    fn node(&self) -> &SyntaxNode;
}

macro_rules! impl_ast {
    ($name:ident, $kind:pat $(if $guard:expr)?) => {
        impl $crate::prelude::AstNode for $name {
            fn can_cast(node: &$crate::prelude::SyntaxData) -> bool {
                matches!(node.kind(), $kind $(if $guard)?)
            }

            fn cast(tn: $crate::prelude::SyntaxNode) -> Result<Self, $crate::prelude::SyntaxNode> {
                if Self::can_cast(&tn.node) {
                    Ok(Self(tn))
                } else {
                    Err(tn)
                }
            }

            fn node(&self) -> &$crate::prelude::SyntaxNode {
                &self.0
            }
        }
    };
}

pub(crate) use impl_ast;
