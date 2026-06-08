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

    /// a mention
    ///
    /// eg. `<@user-uuid-here>`, `<&role-uuid-here>`, or `<#channel-uuid-here>`
    // NOTE: do i include`@everyone`?
    Mention,

    /// a unicode emoji character
    // unsure if i should include this?
    UnicodeEmoji,

    /// a custom emoji
    ///
    /// eg. `<:name:emoji-uuid-here>`
    CustomEmoji,

    /// a newline character (`\n`)
    Newline,

    // markdown syntax
    /// list item prefix syntax
    ListPrefix,

    /// the language of a code block
    CodeblockLang,

    /// check for a ListTask item
    // NOTE: is this part of markdown syntax..?
    TaskCheck,

    /// the alignment to use for a table colum
    ///
    /// eg. `:----`, `-`, `:--:`
    TableAlignment,

    /// the `#` chars after
    ///
    /// the space after the hashes is `Syntax` rather than `HeaderHashes`
    HeaderHashes,
}

/// the kind of an error node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// missing a closing paren
    Closing,
}

impl BlockKind {
    pub fn is_header(&self) -> bool {
        matches!(
            self,
            Self::Header1
                | Self::Header2
                | Self::Header3
                | Self::Header4
                | Self::Header5
                | Self::Header6
        )
    }
}

impl TextKind {
    /// whether this is part of markdown syntax
    pub fn is_syntax(&self) -> bool {
        !matches!(
            self,
            Self::Text
                | Self::Url
                | Self::Mention
                | Self::UnicodeEmoji
                | Self::CustomEmoji
                | Self::Newline
        )
    }
}

impl NodeKind {
    /// whether this is a parse error
    pub fn is_error(&self) -> bool {
        // NOTE: error nodes dont exist yet
        false
    }

    /// whether this node appears in inline markdown
    pub fn is_inline(&self) -> bool {
        matches!(self, NodeKind::Inline(_) | NodeKind::Text(_))
    }

    /// whether this node appears at the block level
    pub fn is_block(&self) -> bool {
        matches!(self, NodeKind::Block(_) | NodeKind::Document)
    }
}
