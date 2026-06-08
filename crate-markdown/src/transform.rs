use rowan::GreenNodeBuilder;

use crate::{ast::inline::Inline, prelude::*};

/// a transformation that can be applied to a syntax tree
pub trait Transform {
    /// apply the transformation to a syntax node, returning the new green root node
    fn apply(&self, root: SyntaxNode) -> GreenNode;

    // PERF: use node cache
    // fn apply(&self, root: SyntaxNode, cache: &mut NodeCache) -> GreenNode;
}

// // for combining (and maybe fusing) multiple transforms?
// pub struct Pipeline {}

pub struct StripEmoji {
    pub allowed: Vec<Uuid>,
}

impl Transform for StripEmoji {
    // fn apply(&self, root: SyntaxNode, cache: &mut NodeCache) -> GreenNode {
    fn apply(&self, root: SyntaxNode) -> GreenNode {
        let mut builder = GreenNodeBuilder::new();
        self.walk(root.into(), &mut builder);
        builder.finish()
    }
}

impl StripEmoji {
    fn walk(&self, el: SyntaxElement, builder: &mut GreenNodeBuilder) {
        if let Some(inline) = Inline::cast(el.clone()) {
            if let Inline::CustomEmoji(emoji) = inline {
                let data = emoji.parse();
                if !self.allowed.contains(&data.id) {
                    let text = format!(":{}:", data.name);
                    builder.token(
                        rowan::SyntaxKind::from(NodeKind::Text(TextKind::Text)),
                        &text,
                    );
                    return;
                }
            }
        }

        match el {
            SyntaxElement::Node(node) => {
                builder.start_node(node.kind().into());
                for child in node.children_with_tokens() {
                    self.walk(child, builder);
                }
                builder.finish_node();
            }
            SyntaxElement::Token(token) => {
                builder.token(token.kind().into(), token.text());
            }
        }
    }
}
