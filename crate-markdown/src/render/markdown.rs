use crate::prelude::*;

/// render back to markdown
///
/// this is an identity transformation
#[derive(Debug, Default)]
pub struct MarkdownRenderer {
    // prevent people from constructing manually
    _a: (),
}

impl Renderer for MarkdownRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        // NOTE: maybe i should manually/explicitly walk the tree?
        q.get_root().to_string()
    }
}
