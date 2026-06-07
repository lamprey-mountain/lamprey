pub mod ast;
pub mod grammar;
pub mod parser;
pub mod render;
pub mod tokenizer;
pub mod tree;
pub mod util;

#[cfg(test)]
mod tests;

// for internal use
pub(crate) mod prelude {
    pub use crate::util::Span;

    #[cfg(feature = "wasm")]
    pub use wasm_bindgen::prelude::*;

    #[cfg(not(feature = "parallel"))]
    pub type Ref<T> = std::rc::Rc<T>;

    #[cfg(feature = "parallel")]
    pub type Ref<T> = std::sync::Arc<T>;

    // #[cfg(not(feature = "parallel"))]
    // pub type Weak<T> = std::rc::Weak<T>;

    // #[cfg(feature = "parallel")]
    // pub type Weak<T> = std::sync::Weak<T>;

    // TODO: doc comment
    pub type Len = u16;
}
