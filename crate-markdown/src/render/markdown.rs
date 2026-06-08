use crate::prelude::*;

/// render back to markdown
///
/// this is an identity transformation
pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        todo!()
    }
}
