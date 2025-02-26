use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{emoji::Emoji, util::Time, RoleId, RoomId, ThreadId, UserId};

/// a language
// TODO: determine which format to use. probably either ietf bcp-47 or a custom enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Language(pub String);

/// any piece of text intended for humans to read; only has light formatting
pub struct PlainText(pub String);

mod parse;
mod render;

/// any piece of text intended for humans to read; may be formatted (eg. messages)
/// uses my as of yet unspecced (and unnamed) text format
/// formatted inline text
// TODO: make generic over KnownTag? and string format?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text<'a>(pub Vec<Span<'a>>);

/// a single span of parsed text
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span<'a> {
    /// a raw string
    Text(Cow<'a, str>),

    /// a formatting tag
    Tag(Tag<'a>),
}

/// a piece of tagged text. tags have a type and multiple parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag<'a> {
    pub name: Cow<'a, str>,
    pub params: Vec<Text<'a>>,
}

/// a container that holds a string. pretty much a workaround for rust's
/// borrowing/lifetimes etc here
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRaw(Box<str>);

impl TextRaw {
    pub fn parse(&self) -> Text {
        Text::parse(&self.0)
    }

    pub fn into_inner(self) -> Box<str> {
        self.0
    }
}

impl<'a> From<&'a TextRaw> for Text<'a> {
    fn from(value: &'a TextRaw) -> Self {
        Text::parse(&value.0)
    }
}

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

impl From<String> for Language {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod test {
    use crate::text::Text;
    use std::fmt::Write;

    #[test]
    fn test_html() {
        let t = Text::parse("hello ~b{world} ~a{https://example.com/}{text} ~em{nested ~b{text}}");
        let mut s = String::new();
        write!(s, "{}", t.as_html()).unwrap();
        assert_eq!(
            s,
            "hello <b>world</b> <a href=\"https://example.com/\">text</a> <em>nested <b>text</b></em>"
        );
    }

    #[test]
    fn test_plain() {
        let t = Text::parse("hello ~b{world} ~a{https://example.com/}{text} ~em{nested ~b{text}}");
        let mut s = String::new();
        write!(s, "{}", t.as_plain()).unwrap();
        assert_eq!(s, "hello world text (https://example.com/) nested text");
    }

    #[test]
    fn test_tagged_text() {
        let t = Text::parse("hello ~b{world} ~a{https://example.com/}{text} ~em{nested ~b{text}}");
        let mut s = String::new();
        write!(s, "{}", t.as_tagged_text()).unwrap();
        assert_eq!(
            s,
            "hello ~b{world} ~a{https://example.com/}{text} ~em{nested ~b{text}}"
        );
    }
}
