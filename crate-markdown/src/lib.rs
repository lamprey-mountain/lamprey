pub mod ast;
// pub mod grammar;
pub mod parser;
pub mod query;
pub mod render;
pub mod tokenizer;
pub mod tree;
pub mod util;

#[cfg(test)]
mod tests;

pub use parser::Parser;

// for internal use
pub(crate) mod prelude {
    pub use crate::ast::AstNode;
    pub use crate::query::Queryable;
    pub use crate::render::Renderer;
    pub use crate::tokenizer::TokenKind;
    pub use crate::tree::kind::{BlockKind, ErrorKind, InlineKind, NodeKind, TextKind};
    pub use crate::tree::node::{GreenNode, SyntaxNode, SyntaxToken};
    pub use crate::tree::Tree;
    pub use crate::util::Span;

    pub use uuid::Uuid;

    #[cfg(feature = "wasm")]
    pub use wasm_bindgen::prelude::*;

    #[cfg(not(feature = "parallel"))]
    pub type Ref<T> = std::rc::Rc<T>;

    #[cfg(feature = "parallel")]
    pub type Ref<T> = std::sync::Arc<T>;

    #[cfg(not(feature = "parallel"))]
    pub type Weak<T> = std::rc::Weak<T>;

    #[cfg(feature = "parallel")]
    pub type Weak<T> = std::sync::Weak<T>;

    /// the type of a string's length
    pub type Len = u16;
}
