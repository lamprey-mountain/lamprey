use crate::parser::Parsed;
use crate::prelude::*;
use crate::query::Queryable;

mod html;
mod markdown;
mod plaintext;

pub use html::HtmlRenderer;
pub use markdown::MarkdownRenderer;
pub use plaintext::PlaintextRenderer;

/// A renderer that converts a markdown syntax tree to a specific output format.
pub trait Renderer {
    /// The output type produced by this renderer.
    type Output;

    /// Render a syntax tree to the output format.
    fn render<Q: Queryable>(&self, q: Q) -> Self::Output;
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Parsed {
    /// render to html
    pub fn to_html(&self) -> String {
        HtmlRenderer.render(self.tree())
    }

    /// render to markdown (identity)
    pub fn to_markdown(&self) -> String {
        MarkdownRenderer.render(self.tree())
    }

    /// render to plaintext, stripping any formatting
    pub fn to_plain(&self) -> String {
        (PlaintextRenderer {}).render(self.tree())
    }
}
