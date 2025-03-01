use std::borrow::{Borrow, Cow};

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// a language
// TODO: determine which format to use. probably either ietf bcp-47 or a custom enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Language(pub String);

/// any piece of text intended for humans to read; only has light formatting
pub struct PlainText(pub String);

pub mod parse;
pub mod render;
pub mod tags;

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

/// an owned Text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedText(Text<'static>);

// ======= impls =======

impl Borrow<Text<'static>> for OwnedText {
    fn borrow(&self) -> &Text<'static> {
        &self.0
    }
}

impl Text<'static> {
    /// get an OwnedText from this text
    pub fn to_owned(&self) -> OwnedText {
        OwnedText(self.to_static())
    }
}

impl AsRef<Text<'static>> for OwnedText {
    fn as_ref(&self) -> &Text<'static> {
        &self.0
    }
}

impl From<Text<'_>> for OwnedText {
    fn from(value: Text<'_>) -> Self {
        Self(value.to_static())
    }
}

impl From<OwnedText> for Text<'_> {
    fn from(value: OwnedText) -> Self {
        value.0
    }
}

impl Tag<'_> {
    pub fn to_static(&self) -> Tag<'static> {
        Tag {
            name: Cow::Owned(self.name.clone().into_owned()),
            params: self.params.iter().map(|p| p.to_static()).collect(),
        }
    }
}

impl Span<'_> {
    pub fn to_static(&self) -> Span<'static> {
        match self {
            Span::Text(cow) => Span::Text(Cow::Owned(cow.clone().into_owned())),
            Span::Tag(tag) => Span::Tag(tag.to_static()),
        }
    }
}

impl Text<'_> {
    pub fn to_static(&self) -> Text<'static> {
        Text(self.0.iter().map(|p| p.to_static()).collect())
    }
}

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

    #[test]
    fn test_markdown() {
        let t = Text::parse("hello ~b{world} ~a{https://example.com/}{text} ~em{nested ~b{text}}");
        let mut s = String::new();
        write!(s, "{}", t.as_markdown()).unwrap();
        assert_eq!(
            s,
            "hello **world** [text](https://example.com/) *nested **text***"
        );
    }

    #[test]
    fn test_escape() {
        assert_eq!(
            Text::parse("~{~~~}~~~}~b{a~}b}~{~}").as_html().to_string(),
            "{~}~}<b>a}b</b>{}"
        );
    }

    #[test]
    fn test_bad_start_brace() {
        assert_eq!(
            Text::parse("{foo ~b{bar}").as_html().to_string(),
            "{foo <b>bar</b>"
        );
    }

    #[test]
    fn test_bad_end_brace() {
        assert_eq!(
            Text::parse("~b{foo}} bar").as_html().to_string(),
            "<b>foo</b> bar"
        );
    }

    #[test]
    fn test_bad_eof() {
        assert_eq!(
            Text::parse("foo ~b{bar").as_html().to_string(),
            "foo <b>bar</b>"
        );
    }
}
