use core::fmt;
use std::fmt::Write;

use url::Url;

use crate::{
    ast::{
        block::{Block, Document},
        inline::{Inline, MentionData},
        list::{List, TaskListMark},
    },
    prelude::*,
    util::sanitize_html::Sanitizable,
};

/// render to html
#[derive(Debug, Default)]
pub struct HtmlRenderer {
    config: Config,
}

#[derive(Debug, Default)]
pub struct Config {
    cdn_url: Option<Url>,
}

#[derive(Debug)]
struct Inner<'a> {
    writer: String,
    config: &'a Config,
}

impl Renderer for HtmlRenderer {
    type Output = String;

    fn render<Q: Queryable>(&self, q: Q) -> Self::Output {
        let node = q.get_root();
        if let Some(doc) = Document::cast(node) {
            let mut r = Inner {
                writer: String::new(),
                config: &self.config,
            };
            r.render(doc).unwrap();
            r.writer
        } else {
            String::new()
        }
    }
}

impl Inner<'_> {
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
                if paragraph.is_empty() {
                    write!(self.writer, "<br />")?;
                } else {
                    write!(self.writer, "<p>")?;
                    for child in paragraph.children() {
                        self.render_inline(child)?;
                    }
                    write!(self.writer, "</p>")?;
                }
            }
            Block::Blockquote(blockquote) => {
                write!(self.writer, "<blockquote>")?;
                for child in blockquote.children() {
                    self.render_block(child)?;
                }
                write!(self.writer, "</blockquote>")?;
            }
            Block::Codeblock(codeblock) => {
                if let Some(lang) = codeblock.language() {
                    write!(self.writer, "<pre><code class=\"language-{lang}\">")?;
                } else {
                    write!(self.writer, "<pre><code>")?;
                }
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
                write!(
                    self.writer,
                    // TODO: configurable rel
                    "<a rel=\"noopener noreferrer nofollow ugc\" href=\"{}\">",
                    link.href().sanitized()
                )?;
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
                write!(self.writer, "{}", text.text().sanitized())?;
            }
            Inline::Mention(mention) => match mention.parse() {
                MentionData::User(u) => write!(
                    self.writer,
                    "<span class=\"mention mention-user\" data-user-id=\"{id}\">@{name}</span>",
                    id = u,
                    name = "Unknown user",
                )?,
                MentionData::Role(r) => write!(
                    self.writer,
                    "<span class=\"mention mention-role\" data-role-id=\"{id}\">@{name}</span>",
                    id = r,
                    name = "Unknown role",
                )?,
                MentionData::Channel(c) => write!(
                    self.writer,
                    "<span class=\"mention mention-channel\" data-channel-id=\"{id}\">#{name}</span>",
                    id = c,
                    name = "unknown channel",
                )?,
                MentionData::Everyone => write!(
                    self.writer,
                    "<span class=\"mention mention-everyone\">@everyone</span>"
                )?,
            },
            Inline::CustomEmoji(e) => {
                let e = e.parse();
                if let Some(cdn_url) = &self.config.cdn_url {
                    write!(
                        self.writer,
                        r#"<img class="emoji custom-emoji" data-emoji-id="{id}" data-emoji-name="{name}" data-emoji-animated="{animated}" src="{cdn_url}/emoji/{id}?size=64" alt=":{name}:" title=":{name}:">"#,
                        id = e.id,
                        name = e.name.sanitized(),
                        animated = e.animated,
                        cdn_url = cdn_url,
                    )?;
                } else {
                    write!(
                        self.writer,
                        r#"<span class="emoji custom-emoji" data-emoji-id="{id}" data-emoji-name="{name}" data-emoji-animated="{animated}">:{name}:</span>"#,
                        id = e.id,
                        name = e.name.sanitized(),
                        animated = e.animated,
                    )?;
                }
            }
            // TODO: verify that this doesnt risk xss
            Inline::UnicodeEmoji(e) => {
                write!(self.writer, "{}", e.text().sanitized())?;
            }
        }

        Ok(())
    }
}
