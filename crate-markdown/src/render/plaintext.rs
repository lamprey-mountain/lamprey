use crate::prelude::*;

/// render to plain text, stripping any and all formatting
pub struct PlaintextRenderer {
    // TODO: allow configuring how to handle links
    // `link text`, `https://url`, or `link text (https://url)`
}

impl Renderer for PlaintextRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, _q: Q) -> Self::Output {
        todo!()
    }
}
