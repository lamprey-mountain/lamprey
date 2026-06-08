use crate::{
    ast::{
        block::{Block, Document, ListKind},
        inline::{Inline, MentionData},
    },
    prelude::*,
};

/// render to html
pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        let node = q.get_root();
        if let Some(doc) = Document::cast(node) {
            self.render(doc)
        } else {
            // TODO: handle error
            String::new()
        }
    }
}

// PERF: write using std::fmt::Write instead of using strings
impl HtmlRenderer {
    fn render(&self, doc: Document) -> String {
        doc.children()
            .map(|block| self.render_block(block))
            .collect()
    }

    fn render_block(&self, block: Block) -> String {
        match block {
            Block::Header(header) => {
                let level = header.level();
                format!(
                    "<h{}>{}</h{}>",
                    level,
                    header
                        .children()
                        .map(|b| self.render_block(b))
                        .collect::<String>(),
                    level
                )
            }
            Block::Paragraph(paragraph) => {
                format!(
                    "<p>{}</p>",
                    paragraph
                        .children()
                        .map(|i| self.render_inline(i))
                        .collect::<String>()
                )
            }
            Block::Blockquote(blockquote) => {
                format!(
                    "<blockquote>{}</blockquote>",
                    blockquote
                        .children()
                        .map(|b| self.render_block(b))
                        .collect::<String>()
                )
            }
            Block::Codeblock(codeblock) => {
                format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>",
                    codeblock.language().unwrap_or_else(|| "text".to_string()),
                    // TODO: render codeblock children since .text() doesnt handle escapes
                    codeblock.syntax().text().to_string()
                )
            }
            Block::List(list) => {
                let tag = match list.kind() {
                    ListKind::Ordered => "ol",
                    ListKind::Unordered => "ul",
                    // TODO: task list rendering
                    ListKind::Task => "ul",
                };
                format!(
                    "<{}>{}</{}>",
                    tag,
                    list.items()
                        .map(|item| self.render_block(Block::ListItem(item)))
                        .collect::<String>(),
                    tag
                )
            }
            Block::ListItem(list_item) => {
                format!(
                    "<li>{}</li>",
                    list_item
                        .content()
                        .map(|b| self.render_block(b))
                        .collect::<String>()
                )
            }
        }
    }

    fn render_inline(&self, inline: Inline) -> String {
        match inline {
            Inline::Strong(strong) => format!(
                "<strong>{}</strong>",
                strong
                    .children()
                    .map(|i| self.render_inline(i))
                    .collect::<String>()
            ),
            Inline::Emphasis(emphasis) => format!(
                "<em>{}</em>",
                emphasis
                    .children()
                    .map(|i| self.render_inline(i))
                    .collect::<String>()
            ),
            Inline::Link(link) => format!(
                "<a href=\"{}\">{}</a>",
                // TODO: escape
                link.href(),
                link.children()
                    .map(|i| self.render_inline(i))
                    .collect::<String>()
            ),
            Inline::Spoiler(spoiler) => format!(
                "<span class=\"spoiler\">{}</span>",
                spoiler
                    .children()
                    .map(|i| self.render_inline(i))
                    .collect::<String>()
            ),
            Inline::Code(code) => format!(
                "<code>{}</code>",
                code.children()
                    .map(|i| self.render_inline(i))
                    .collect::<String>()
            ),
            // TODO: escape
            Inline::Text(text) => text.text(),
            // TODO: custom html for mentions?
            // maybe make this configurable
            Inline::Mention(mention) => match mention.parse() {
                MentionData::User(u) => format!("@{}", u),
                MentionData::Role(r) => format!("@{}", r),
                MentionData::Channel(c) => format!("#{}", c),
                MentionData::Everyone => "@everyone".to_string(),
            },
            // TODO: custom html for custom emoji?
            // maybe make this configurable
            Inline::CustomEmoji(e) => format!(":{}:", e.parse().name),
            // TODO: verify that this doesnt risk xss
            Inline::UnicodeEmoji(e) => e.text(),
        }
    }
}
