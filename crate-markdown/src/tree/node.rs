use crate::prelude::*;

// NOTE: is u32 correct here?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeIndex(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    kind: NodeKind,
    span: Span,
    children: Vec<NodeIndex>,
}

impl Node {
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// root node
    Document,

    Block(BlockKind),
    Inline(InlineKind),
    Text(TextKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Code,
    Emphasis,
    Strong,
    Link,
    Autolink,
    Strikethrough,
    Spoiler,
    TableHeader,
    TableValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineKind {
    Code,
    Emphasis,
    Strong,
    Link,
    Autolink,
    Strikethrough,
    Spoiler,
    TableHeader,
    TableValue,
}

/// the kind of a text fragment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// a url
    Url,

    /// arbitrary text content
    Text,

    /// other markdown syntax
    Syntax,

    /// the language of a code block
    CodeblockLang,

    /// a mention
    Mention,

    /// check for a ListTask item
    TaskCheck,

    UnicodeEmoji,
    CustomEmoji,
}

/// the kind of an error node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// missing a closing paren
    Closing,
}

impl NodeKind {
    /// whether this is markdown syntax
    pub fn is_syntax(&self) -> bool {
        todo!()
    }

    /// whether this is a parse error
    pub fn is_error(&self) -> bool {
        todo!()
    }

    /// whether this node appears in inline markdown
    pub fn is_inline(&self) -> bool {
        todo!()
    }

    /// whether this node appears at the block level
    pub fn is_block(&self) -> bool {
        todo!()
    }
}
