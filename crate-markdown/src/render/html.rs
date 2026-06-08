use crate::{
    ast::{
        block::{Block, Document},
        inline::Inline,
    },
    prelude::*,
};

/// render to html
pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        let node = q.get_root();
        // convert node to Document
        // self.render(root)
        todo!()
    }
}

impl HtmlRenderer {
    fn render(&self, doc: Document) -> String {
        todo!()
    }

    fn render_block(&self, block: Block) -> String {
        match block {
            Block::Header(header) => todo!(),
            Block::Paragraph(paragraph) => todo!(),
            Block::Blockquote(blockquote) => todo!(),
            Block::Codeblock(codeblock) => todo!(),
            Block::ListItem(list_item) => todo!(),
        }
    }

    fn render_inline(&self, inline: Inline) -> String {
        match inline {
            Inline::Strong(strong) => todo!(),
            Inline::Emphasis(emphasis) => todo!(),
            Inline::Link(link) => todo!(),
            Inline::Spoiler(spoiler) => todo!(),
            Inline::Code(code) => todo!(),
            Inline::Text(text) => todo!(),
            Inline::Mention(mention) => todo!(),
            Inline::CustomEmoji(custom_emoji) => todo!(),
            Inline::UnicodeEmoji(unicode_emoji) => todo!(),
        }
    }
}
