pub mod block;
pub mod inline;
pub mod table;

pub use rowan::ast::AstNode;

macro_rules! impl_ast {
    ($name:ident, $kind:pat $(if $guard:expr)?) => {
        impl $crate::ast::AstNode for $name {
            type Language = $crate::tree::node::MarkdownLanguage;

            fn can_cast(kind: $crate::prelude::NodeKind) -> bool {
                matches!(kind, $kind $(if $guard)?)
            }

            fn cast(node: $crate::prelude::SyntaxNode) -> Option<Self> {
                if Self::can_cast(node.kind()) {
                    Some(Self(node))
                } else {
                    None
                }
            }

            fn syntax(&self) -> &$crate::prelude::SyntaxNode {
                &self.0
            }
        }
    };
}

pub(crate) use impl_ast;
