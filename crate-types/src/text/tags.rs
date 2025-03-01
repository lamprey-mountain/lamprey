use url::Url;

use super::{Tag, Text};
use crate::{emoji::Emoji, util::Time, RoleId, RoomId, ThreadId, UserId};

// TODO: stronger typing
// some of these could have less cloning
/// currently supported tags
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownTag<'a> {
    /// bold text
    Bold(Text<'a>),

    /// emphasized
    Emphasis(Text<'a>),

    /// subscript (may be removed?)
    Sub(Text<'a>),

    /// superscript (may be removed?)
    Sup(Text<'a>),

    /// strikethrough
    Strikethrough(Text<'a>),

    /// link (optional custom text)
    Link(Url, Option<Text<'a>>),

    /// inline code (optional programming language)
    Code(Text<'a>, Option<String>),

    /// spoiler (optional reason)
    Spoiler(Text<'a>, Option<String>),

    /// keyboard shortcut
    Keyboard(Text<'a>),

    /// abbreviation
    Abbr(Text<'a>, Text<'a>),

    // math/latex (how do i standardize this?)
    Math(&'a str),

    /// custom emoji
    Emoji(Emoji),

    /// timestamp
    Time(Time, TimeFormat),

    Mention(MentionTag),
    // Document(DocumentTag<'a>),
    // Interactive(InteractiveTag<'a>),
}

// /// only usable in larger documents
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum DocumentTag<'a> {
//     /// footnote/sidenote
//     Aside(Box<Block<'a>>),
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionTag {
    /// mention a user
    MentionUser(UserId),

    /// mention/link a room
    MentionRoom(RoomId),

    /// mention/link a thread
    MentionThread(ThreadId),

    /// mention everyone with a role
    MentionRole(RoleId),

    /// mention everyone in the room
    MentionAllRoom,

    /// mention everyone in the thread
    MentionAllThread,
}

/// how the time should be displayed
// also might be useful to have a duration format?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeFormat {
    TimeShort,
    TimeLong,
    DateShort,
    DateLong,
    DateTimeShort,
    DateTimeLong,
    Relative,
}

impl<'a> TryFrom<Tag<'a>> for KnownTag<'a> {
    type Error = ();

    fn try_from(value: Tag<'a>) -> Result<Self, Self::Error> {
        match (&*value.name, value.params.as_slice()) {
            ("b", [t]) => Ok(KnownTag::Bold(t.clone())),
            ("em", [t]) => Ok(KnownTag::Emphasis(t.clone())),
            ("a", [l]) => Ok(KnownTag::Link(
                l.as_plain().to_string().parse().map_err(|_| ())?,
                None,
            )),
            ("a", [l, t]) => Ok(KnownTag::Link(
                l.as_plain().to_string().parse().map_err(|_| ())?,
                Some(t.clone()),
            )),
            ("sub", [t]) => Ok(KnownTag::Sub(t.clone())),
            ("sup", [t]) => Ok(KnownTag::Sup(t.clone())),
            ("s", [t]) => Ok(KnownTag::Strikethrough(t.clone())),
            ("code", [t]) => Ok(KnownTag::Code(t.clone(), None)),
            ("code", [t, l]) => Ok(KnownTag::Code(t.clone(), Some(l.as_plain().to_string()))),
            _ => Err(()),
        }
    }
}

/// block level formatting (WIP)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<'a> {
    /// inline text, can be a plain string
    Text(Text<'a>),

    H1(Text<'a>),
    H2(Text<'a>),
    H3(Text<'a>),
    H4(Text<'a>),
    H5(Text<'a>),
    H6(Text<'a>),
    Blockquote(Text<'a>),
    Code(Text<'a>),
    ListUnordered(Vec<Text<'a>>),
    ListOrdered(Vec<Text<'a>>),
    ListDefinition(Vec<(Text<'a>, Text<'a>)>),
    ListCheckable(Vec<(Text<'a>, bool)>),
    Table(Vec<Vec<Text<'a>>>),
    Math(&'a str),
    // Interactive(BlockInteractive),
}

// idk about *any* of these, just throwing random ideas out here
// i'm probably not going to implement them

// /// interactive, probably will be limited to bots
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum BlockInteractive<'a> {
//     /// a clickable button
//     Button(Text<'a>, ButtonStyle),

//     /// a text input
//     Input(Text<'a>, InputStyle),

//     /// collapseable summary and details
//     Details(Text<'a>, Box<Block>),

//     Radio,
//     Checkbox,
//     Form,
// }

// #[derive(Debug, Default, Clone, PartialEq, Eq)]
// pub enum ButtonStyle {
//     #[default]
//     Default,
//     Primary,
//     Danger,
// }

// #[derive(Debug, Default, Clone, PartialEq, Eq)]
// pub enum InputStyle {
//     #[default]
//     Singleline,

//     // Multiline,
//     // RichText,
//     // Url,
//     // Time,
//     // Date,
//     // DateTime,
//     // Number,
//     // File,
//     // Color,
//     // Search,
//     // Select,

//     // User,
//     // MemberThread,
//     // MemberRoom,
//     // Room,
//     // Message,
//     // Thread,
// }

// /// for layout
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum LayoutBlock<'a> {
//     /// footnote/sidenote
//     Aside(Box<Block<'a>>),
//     Row(Vec<Block<'a>, StyleFlex>),
//     Column(Vec<Block<'a>, StyleFlex>),
//     Grid(Vec<Block<'a>>, StyleGrid),
//     Box(Vec<Block<'a>>, StyleBox),
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocks<'a>(Vec<Block<'a>>);

#[allow(unused)]
// i really should stop trying to overengineer this stuff
// maybe i'll do a v1 static/no interaction version then a v2 format with interactivity
// so that i at least have *some* way of formatting text for now
//
// another note: i want to let people add custom styles for user/room profiles.
// i'd use css, but its kind of a pain to sanitize and scope properly. so i
// might *also* end up making my own styling as well...
//
// plus theres the whole "maybe i want to let people run arbitrary code" thing.
// might as well bite the bullet and figure out a way to safely let people do
// *anything*, instead of incrementally/manually adding more and more features as
// people ask for them.
//
/// the planning never ceases
mod more_random_ideas {
    use crate::text::{Language, Text};

    /// a reference to another Block in a Document
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BlockIdx(u64);

    /// block level formatting (WIP, new version?)
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Block<'a> {
        /// usually the root level container. allows combining multiple other block level elements.
        Container(Vec<u64>),

        /// inline text, can be a plain string
        Text(Text<'a>),

        H1(Text<'a>),
        H2(Text<'a>),
        H3(Text<'a>),
        H4(Text<'a>),
        H5(Text<'a>),
        H6(Text<'a>),
        Blockquote(BlockIdx),
        Code(Language, Text<'a>),
        ListUnordered(Vec<BlockIdx>),
        ListOrdered(Vec<BlockIdx>),
        ListDefinition(Vec<(BlockIdx, BlockIdx)>),
        ListCheckable(Vec<(BlockIdx, bool)>),
        Table(Vec<Vec<BlockIdx>>),
        Math(&'a str),

        /// for templates, a hole thats filled in later
        Data(Box<str>),

        /// causes/emits effects
        // bubble or capture?
        Effect(Effect),

        /// catches and transforms/handles effects
        // TODO: ?????
        // this is becoming very overcomplicated very quickly
        // i also want to limit the amount of "special" components and push more
        // stuff into "userspace", which is absolutely going to increase the
        // complexity here
        Handle(Input, Output),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Input;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Output;

    // also should look at https://adaptivecards.io/ for ideas
    // ...though i'll probably not copy it exactly
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Effect {
        /// send an event to whoever sent this
        Event(Box<str>),

        /// do something to another thing
        Something(BlockIdx),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Document {
        /// is None, is assumed to be (thread, room, user)'s language
        pub lang: Option<Language>,

        /// first block is the root node. is a graph, not a tree.
        pub blocks: Vec<Block<'static>>,

        /// fills in Data
        pub data: std::collections::HashMap<Box<str>, BlockIdx>,
    }
}
