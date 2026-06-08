use crate::parser::Parsed;
use crate::prelude::*;
use crate::query::Queryable;

mod html;
mod markdown;
mod plaintext;

pub use html::HtmlRenderer;
pub use markdown::MarkdownRenderer;
pub use plaintext::PlaintextRenderer;

mod refactor_these_types {
    use crate::prelude::*;

    /// a decoration that can be applied to the markdown source
    // TODO: better types
    pub struct Decoration {
        pub span: Span,
        pub attrs: DecorationAttrs,
        // options?: { inclusiveStart?: boolean; inclusiveEnd?: boolean };
    }

    pub struct DecorationAttrs {
        // consider making strings &' static str
        pub node_name: String,
        pub class: String,
        pub style: String,
    }

    // pub struct DecorationAttr {}
    // pub enum DecorationClass {
    //     Syn,
    //     SynCodeLang,
    //     Em,
    //     B,
    //     Spoiler,
    //     Link,
    // }

    // impl DecorationAttr {
    //     /// get the class name for this node
    //     pub fn class_name(&self) -> &str {
    //         todo!()
    //     }
    // }

    // pub enum DecorationEvent {
    //     Add,
    //     Remove,
    // }
}

pub use refactor_these_types::*;

/// A renderer that converts a markdown syntax tree to a specific output format.
pub trait Renderer {
    /// The output type produced by this renderer.
    type Output;

    /// Render a syntax tree to the output format.
    fn render<Q: Queryable>(&self, q: Q) -> Self::Output;
}

// TODO: impl for Queryable?
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Parsed {
    /// render to html
    pub fn to_html(&self) -> String {
        HtmlRenderer.render(self.tree_clone())
    }

    /// render to markdown (identity)
    pub fn to_markdown(&self) -> String {
        MarkdownRenderer.render(self.tree_clone())
    }

    /// render to plaintext, stripping any formatting
    pub fn to_plain(&self) -> String {
        (PlaintextRenderer {}).render(self.tree_clone())
    }
}
