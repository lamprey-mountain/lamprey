use crate::{
    ast::{
        block::{Block, Document},
        inline::{Inline, MentionData},
        list::List,
    },
    prelude::*,
};

/// render to plain text, stripping any and all formatting
pub struct PlaintextRenderer;

impl Renderer for PlaintextRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        let node = q.get_root();
        if let Some(doc) = Document::cast(node) {
            self.render(doc)
        } else {
            String::new()
        }
    }
}

impl PlaintextRenderer {
    fn render(&self, doc: Document) -> String {
        doc.children()
            .map(|block| self.render_block(block))
            .collect::<Vec<String>>()
            .join("\n\n")
    }

    fn render_block(&self, block: Block) -> String {
        match block {
            Block::Header(header) => header
                .children()
                .map(|b| self.render_inline(b))
                .collect::<String>(),
            Block::Paragraph(paragraph) => paragraph
                .children()
                .map(|i| self.render_inline(i))
                .collect::<String>(),
            Block::Blockquote(blockquote) => blockquote
                .children()
                .map(|b| self.render_block(b))
                .collect::<Vec<String>>()
                .join("\n"),
            Block::Codeblock(codeblock) => codeblock
                .content()
                .map(|i| self.render_inline(i))
                .collect::<String>(),
            Block::List(list) => match list {
                List::Ordered(l) => l
                    .items()
                    .enumerate()
                    .map(|(i, item)| {
                        format!(
                            "{}. {}",
                            i + 1,
                            item.children()
                                .map(|node| self.render_inline(node))
                                .collect::<String>()
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n"),
                List::Unordered(l) => l
                    .items()
                    .map(|item| {
                        format!(
                            "- {}",
                            item.children()
                                .map(|node| self.render_inline(node))
                                .collect::<String>()
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n"),
                List::Tasks(l) => l
                    .items()
                    .map(|item| {
                        format!(
                            "- [{}] {}",
                            item.mark(),
                            item.children()
                                .map(|node| self.render_inline(node))
                                .collect::<String>()
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n"),
            },
            Block::Table(_table) => String::new(), // TODO: render table to plaintext
        }
    }

    fn render_inline(&self, inline: Inline) -> String {
        match inline {
            Inline::Strong(strong) => strong.children().map(|i| self.render_inline(i)).collect(),
            Inline::Emphasis(emphasis) => {
                emphasis.children().map(|i| self.render_inline(i)).collect()
            }
            Inline::Link(link) => {
                let text: String = link.children().map(|i| self.render_inline(i)).collect();
                format!("{} ({})", text, link.href())
            }
            Inline::Spoiler(spoiler) => spoiler.children().map(|i| self.render_inline(i)).collect(),
            Inline::Strikethrough(s) => s.children().map(|i| self.render_inline(i)).collect(),
            Inline::Code(code) => code.children().map(|i| self.render_inline(i)).collect(),
            Inline::Timestamp(timestamp) => {
                // TODO: render timestamp from rust?
                let t = timestamp.time();
                format!("<t:{}>", t)
            }
            Inline::Text(text) => {
                // ignore syntax tokens
                if matches!(text.syntax().kind(), NodeKind::Text(TextKind::Syntax)) {
                    "".to_string()
                } else {
                    text.text()
                }
            }
            Inline::Mention(mention) => match mention.parse() {
                MentionData::User(u) => format!("@{}", u),
                MentionData::Role(r) => format!("@{}", r),
                MentionData::Channel(c) => format!("#{}", c),
                MentionData::Everyone => "@everyone".to_string(),
            },
            Inline::CustomEmoji(e) => format!(":{}:", e.parse().name),
            Inline::UnicodeEmoji(e) => e.text(),
        }
    }
}
