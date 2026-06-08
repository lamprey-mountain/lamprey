use crate::prelude::*;

/// render to html
pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        let tn = q.get_tree_node();
        todo!("traverse with cursor")
        // self.render_node(root)
    }
}

impl HtmlRenderer {
    // fn render_node(&self, node: TreeNode) -> String {
    //     match Block::cast(node.clone()) {
    //         Ok(Block::Document(doc)) => doc.children().map(|c| self.render_node(TreeNode { tree: Ref::clone(&node.tree), node: c.node().node.clone() })).collect(),
    //         Ok(Block::Paragraph(p)) => format!("<p>{}</p>", p.children().map(|c| self.render_block_child(c)).collect::<String>()),
    //         Ok(Block::Header(h)) => format!("<h{}>{}</h{}>", h.level(), h.children().map(|c| self.render_block_child(c)).collect::<String>(), h.level()),
    //         _ => self.render_inline(node),
    //     }
    // }

    // fn render_block_child(&self, block: Block) -> String {
    //     // This is a bit simplified, assuming blocks contain inline content or other blocks
    //     match block {
    //         _ => self.render_node(TreeNode { tree: Ref::clone(&block.node().tree), node: block.node().node.clone() }),
    //     }
    // }

    // fn render_inline(&self, node: TreeNode) -> String {
    //     if let Ok(inline) = Inline::cast(node.clone()) {
    //         match inline {
    //             Inline::Strong(s) => format!("<strong>{}</strong>", s.children().map(|c| self.render_inline_child(c)).collect::<String>()),
    //             Inline::Emphasis(e) => format!("<em>{}</em>", e.children().map(|c| self.render_inline_child(c)).collect::<String>()),
    //             Inline::Text(t) => t.text().to_string(), // Need to escape HTML entities
    //             Inline::Code(c) => format!("<code>{}</code>", c.text()),
    //             _ => node.text().to_string(),
    //         }
    //     } else {
    //         node.text().to_string()
    //     }
    // }

    // fn render_inline_child(&self, inline: Inline) -> String {
    //     match inline {
    //         Inline::Strong(s) => format!("<strong>{}</strong>", s.children().map(|c| self.render_inline_child(c)).collect::<String>()),
    //         Inline::Emphasis(e) => format!("<em>{}</em>", e.children().map(|c| self.render_inline_child(c)).collect::<String>()),
    //         Inline::Text(t) => t.text().to_string(),
    //         Inline::Code(c) => format!("<code>{}</code>", c.text()),
    //         _ => inline.node().text().to_string(),
    //     }
    // }
}
