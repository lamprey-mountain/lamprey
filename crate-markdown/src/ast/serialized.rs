//! serialized version of the ast for js

use serde::{Deserialize, Serialize};

use crate::ast::{
    block::{Block, Document},
    inline::{Inline, MentionData},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedDocument {
    blocks: Vec<SerializedBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SerializedBlock {
    Header {
        level: u8,
        children: Vec<SerializedInline>,
    },
    Paragraph {
        children: Vec<SerializedInline>,
    },
    Blockquote {
        children: Vec<SerializedBlock>,
    },
    Codeblock {
        language: Option<String>,
        content: String,
    },
    List {
        items: Vec<SerializedBlock>,
    },
    ListItem {
        content: Vec<SerializedBlock>,
    },
    Table {
        header: Vec<Vec<SerializedInline>>,
        rows: Vec<Vec<Vec<SerializedInline>>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SerializedInline {
    Strong {
        children: Vec<SerializedInline>,
    },
    Emphasis {
        children: Vec<SerializedInline>,
    },
    Strikethrough {
        children: Vec<SerializedInline>,
    },
    Link {
        href: String,
        children: Vec<SerializedInline>,
    },
    Spoiler {
        children: Vec<SerializedInline>,
    },
    Code {
        children: Vec<SerializedInline>,
    },
    Text {
        content: String,
    },
    Mention {
        mention: MentionData,
    },
    CustomEmoji {
        animated: bool,
        name: String,
        id: String,
    },
    UnicodeEmoji {
        content: String,
    },
}

impl SerializedDocument {
    pub fn from_document(doc: Document) -> Self {
        Self {
            blocks: doc.children().map(SerializedBlock::from_block).collect(),
        }
    }
}

impl SerializedBlock {
    pub fn from_block(block: Block) -> Self {
        match block {
            Block::Header(h) => Self::Header {
                level: h.level(),
                children: h.children().map(SerializedInline::from_inline).collect(),
            },
            Block::Paragraph(p) => Self::Paragraph {
                children: p.children().map(SerializedInline::from_inline).collect(),
            },
            Block::Blockquote(b) => Self::Blockquote {
                children: b.children().map(SerializedBlock::from_block).collect(),
            },
            Block::Codeblock(c) => Self::Codeblock {
                language: c.language(),
                // TODO: handle inline formatting in codeblocks..?
                content: c
                    .content()
                    .map(|i| i.syntax().to_string())
                    .collect::<String>(),
            },
            Block::List(l) => Self::List {
                items: l
                    .items()
                    .map(|i| Self::from_block(Block::ListItem(i)))
                    .collect(),
            },
            Block::ListItem(li) => Self::ListItem {
                content: li.content().map(SerializedBlock::from_block).collect(),
            },
            Block::Table(t) => Self::Table {
                header: t
                    .header()
                    .map(|r| {
                        r.cells()
                            .map(|c| c.children().map(SerializedInline::from_inline).collect())
                            .collect()
                    })
                    .unwrap_or_default(),
                rows: t
                    .body()
                    .map(|r| {
                        r.cells()
                            .map(|c| c.children().map(SerializedInline::from_inline).collect())
                            .collect()
                    })
                    .collect(),
            },
        }
    }
}

impl SerializedInline {
    pub fn from_inline(inline: Inline) -> Self {
        match inline {
            Inline::Strong(s) => Self::Strong {
                children: s.children().map(Self::from_inline).collect(),
            },
            Inline::Emphasis(e) => Self::Emphasis {
                children: e.children().map(Self::from_inline).collect(),
            },
            Inline::Strikethrough(s) => Self::Strikethrough {
                children: s.children().map(Self::from_inline).collect(),
            },
            Inline::Link(l) => Self::Link {
                href: l.href(),
                children: l.children().map(Self::from_inline).collect(),
            },
            Inline::Spoiler(s) => Self::Spoiler {
                children: s.children().map(Self::from_inline).collect(),
            },
            Inline::Code(c) => Self::Code {
                children: c.children().map(Self::from_inline).collect(),
            },
            Inline::Text(t) => Self::Text { content: t.text() },
            Inline::Mention(m) => Self::Mention { mention: m.parse() },
            Inline::CustomEmoji(e) => {
                let data = e.parse();
                Self::CustomEmoji {
                    animated: data.animated,
                    name: data.name,
                    id: data.id.to_string(),
                }
            }
            Inline::UnicodeEmoji(u) => Self::UnicodeEmoji { content: u.text() },
        }
    }
}
