use core::fmt;
use std::fmt::Write;

use crate::{
    ast::{
        block::{Block, Document},
        inline::{Inline, MentionData},
        list::{List, TaskListMark},
    },
    prelude::*,
};

/// render to html
#[derive(Debug, Default)]
pub struct HtmlRenderer {
    _nope: (), // prevent people from manually constructing this
}

#[derive(Debug, Default)]
struct Inner {
    writer: String,
}

impl Renderer for HtmlRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        let node = q.get_root();
        if let Some(doc) = Document::cast(node) {
            let mut r = Inner::default();
            r.render(doc).unwrap();
            r.writer
        } else {
            // TODO: handle error
            String::new()
        }
    }
}

impl Inner {
    fn render(&mut self, doc: Document) -> fmt::Result {
        for block in doc.children() {
            self.render_block(block)?;
        }
        Ok(())
    }

    fn render_block(&mut self, block: Block) -> fmt::Result {
        match block {
            Block::Header(header) => {
                let level = header.level();
                write!(self.writer, "<h{}>", level)?;
                for child in header.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</h{}>", level)?;
            }
            Block::Paragraph(paragraph) => {
                write!(self.writer, "<p>")?;
                for child in paragraph.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</p>")?;
            }
            Block::Blockquote(blockquote) => {
                write!(self.writer, "<blockquote>")?;
                for child in blockquote.children() {
                    self.render_block(child)?;
                }
                write!(self.writer, "</blockquote>")?;
            }
            Block::Codeblock(codeblock) => {
                write!(
                    self.writer,
                    "<pre><code class=\"language-{}\">",
                    codeblock.language().unwrap_or_default()
                )?;
                for child in codeblock.content() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</code></pre>")?;
            }
            Block::List(list) => match list {
                List::Ordered(l) => {
                    write!(self.writer, "<ol>")?;
                    for item in l.items() {
                        write!(self.writer, "<li>")?;
                        for node in item.children() {
                            self.render_inline(node)?;
                        }
                        write!(self.writer, "</li>")?;
                    }
                    write!(self.writer, "</ol>")?;
                }
                List::Unordered(l) => {
                    write!(self.writer, "<ul>")?;
                    for item in l.items() {
                        write!(self.writer, "<li>")?;
                        for node in item.children() {
                            self.render_inline(node)?;
                        }
                        write!(self.writer, "</li>")?;
                    }
                    write!(self.writer, "</ul>")?;
                }
                List::Tasks(l) => {
                    write!(self.writer, r#"<ul class="task-list">"#)?;
                    for item in l.items() {
                        let checked = if item.mark() == TaskListMark::Complete {
                            "checked"
                        } else {
                            ""
                        };
                        // TODO: research better ways of rendering this?
                        // maybe add config to HtmlRenderer
                        write!(
                            self.writer,
                            r#"<li class="task-item"><input class="task-checkbox" type="checkbox" {} disabled />"#,
                            checked
                        )?;
                        for node in item.children() {
                            self.render_inline(node)?;
                        }
                        write!(self.writer, "</li>")?;
                    }
                    write!(self.writer, "</ul>")?;
                }
            },
            Block::Table(table) => {
                write!(self.writer, "<table>")?;
                let mut rows = table.rows().peekable();

                if let Some(header_row) = rows.next() {
                    write!(self.writer, "<thead><tr>")?;
                    for cell in header_row.cells() {
                        write!(self.writer, "<th>")?;
                        for i in cell.children() {
                            self.render_inline(i)?;
                        }
                        write!(self.writer, "</th>")?;
                    }
                    write!(self.writer, "</tr></thead>")?;
                }

                rows.next(); // skip alignment row

                if rows.peek().is_some() {
                    write!(self.writer, "<tbody>")?;
                    for row in rows {
                        write!(self.writer, "<tr>")?;
                        for cell in row.cells() {
                            write!(self.writer, "<td>")?;
                            for i in cell.children() {
                                self.render_inline(i)?;
                            }
                            write!(self.writer, "</td>")?;
                        }
                        write!(self.writer, "</tr>")?;
                    }
                    write!(self.writer, "</tbody>")?;
                }

                write!(self.writer, "</table>")?;
            }
        }
        Ok(())
    }

    fn render_inline(&mut self, inline: Inline) -> fmt::Result {
        match inline {
            Inline::Strong(strong) => {
                write!(self.writer, "<strong>")?;
                for child in strong.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</strong>")?;
            }
            Inline::Emphasis(emphasis) => {
                write!(self.writer, "<em>")?;
                for child in emphasis.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</em>")?;
            }
            Inline::Link(link) => {
                // TODO: escape
                write!(self.writer, "<a href=\"{}\">", link.href())?;
                for child in link.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</a>")?;
            }
            Inline::Spoiler(spoiler) => {
                write!(self.writer, "<span class=\"spoiler\">")?;
                for child in spoiler.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</span>")?;
            }
            Inline::Strikethrough(s) => {
                write!(self.writer, "<s>")?;
                for child in s.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</s>")?;
            }
            Inline::Code(code) => {
                write!(self.writer, "<code>")?;
                for child in code.children() {
                    self.render_inline(child)?;
                }
                write!(self.writer, "</code>")?;
            }
            Inline::Timestamp(timestamp) => {
                // TODO: render timestamp from rust?
                write!(
                    self.writer,
                    "<time datetime=\"{}\" data-style=\"{}\"></time>",
                    timestamp.time(),
                    timestamp.style()
                )?;
            }
            Inline::Text(text) => {
                // TODO: escape
                write!(self.writer, "{}", text.text())?;
            }
            // TODO: custom html for mentions?
            // maybe make this configurable
            Inline::Mention(mention) => match mention.parse() {
                MentionData::User(u) => write!(self.writer, "@{}", u)?,
                MentionData::Role(r) => write!(self.writer, "@{}", r)?,
                MentionData::Channel(c) => write!(self.writer, "#{}", c)?,
                MentionData::Everyone => write!(self.writer, "@everyone")?,
            },
            // TODO: custom html for custom emoji?
            // maybe make this configurable
            Inline::CustomEmoji(e) => {
                write!(self.writer, ":{}:", e.parse().name)?;
            }
            // TODO: verify that this doesnt risk xss
            Inline::UnicodeEmoji(e) => {
                write!(self.writer, "{}", e.text())?;
            }
        }

        Ok(())
    }
}
