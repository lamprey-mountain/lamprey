use crate::prelude::*;

// NOTE: is u32 correct here?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeIndex(pub(crate) u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    kind: NodeKind,

    // NOTE: maybe i don't want to store Span so that identical nodes can be reused
    span: Span,

    // TEMP: need to create a decent interface for this
    pub(crate) children: Vec<NodeIndex>,
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
    Codeblock,
    Paragraph,
    Blockquote,

    // headers
    Header1,
    Header2,
    Header3,
    Header4,
    Header5,
    Header6,

    // lists
    ListItem,
    ListOrdered,
    ListUnordered,
    ListTasks,

    // tables
    Table,
    // TODO: design types
    // TableRowLine,
    // TableRow,
    // TableAlignmentRow,
    // TableHeader,
    // TableBody,
}

/// the type of an inline node
///
/// inline nodes generally include syntax text, eg. `*` for emphasis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineKind {
    // includes
    Code,
    Emphasis,
    Strong,
    Link, //
    Strikethrough,
    Spoiler,
    TableHeader,
    TableValue,

    /// a url that should be automatically converted into a link
    ///
    /// if they exist, `<` and `>` are included in children as `Syntax`
    Autolink,
}

/// the kind of a text fragment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// arbitrary text content that doesnt match any of the other types
    Text,

    /// a url
    Url,

    /// other markdown syntax
    Syntax,

    /// the language of a code block
    CodeblockLang,

    /// a mention
    ///
    /// eg. `<@user-uuid-here>`, `<&role-uuid-here>`, or `<#channel-uuid-here>`
    // NOTE: do i include`@everyone`?
    Mention,

    /// a custom emoji
    ///
    /// eg. `<:name:emoji-uuid-here>`
    CustomEmoji,

    /// check for a ListTask item
    TaskCheck,

    /// a unicode emoji character
    // unsure if i should include this?
    UnicodeEmoji,

    /// the alignment to use for a table colum
    ///
    /// eg. `:----`, `-`, `:--:`
    TableAlignment,

    /// the `#` chars after
    ///
    /// the space after the hashes is `Syntax` rather than `HeaderHashes`
    HeaderHashes,

    /// a newline character (`\n`)
    Newline,
}

/// the kind of an error node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// missing a closing paren
    Closing,
}

impl BlockKind {
    pub fn is_header(&self) -> bool {
        todo!()
    }
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
