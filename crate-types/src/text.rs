use std::borrow::{Borrow, Cow};

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// a language
// TODO: determine which format to use. probably either ietf bcp-47 or a custom enum.
// does this include programming languages?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Language(pub String);

// TODO: add a way to restrict what formatting can be used where
#[allow(unused)]
mod formatting {
    pub trait TextFormat {}

    /// no formatting
    pub enum TextFormatPlain {}

    /// inline formatting
    pub enum TextFormatInline {}

    /// paragraph, blockquote, code, list formatting
    pub enum TextFormatMore {}

    /// all block (incl heading)
    pub enum TextFormatDocument {}

    // /// all block (incl interactive stuff later)
    // pub enum TextFormatInteractive {}

    impl TextFormat for TextFormatPlain {}
    impl TextFormat for TextFormatInline {}
    impl TextFormat for TextFormatMore {}
    impl TextFormat for TextFormatDocument {}
}

pub mod parse;
pub mod render;
pub mod tags;

// i want to redo this whole text thing, it's a bit of a mess
// eg. to_owned will cause tons of small allocations for strings
// TODO: use Text/OwnedText (or Cow) everywhere

// i might also want to see how backwards compatibility can be added in. maybe
// {} and [] could both be used, but [] isn't displayed if a tag isn't supported.

// other formats?
// "text ~foo{1}{2} more text" vs "text {foo 1 2} more text", would need a way
// to indicate nested blocks due to whitespace "{foo [some text] [more text]}"
// named attrs ~foo{one=hello}{two=world} or ~foo{one=hello, two=world}
// current variable param list system might be a footgun
// someone mentioned that lua table syntax might be okay?
// maybe use () instead of {}, or something else. {} is pretty much arbitrary.

// see https://web.archive.org/web/20120210103745/http://cairnarvon.rotahall.org/misc/sexpcode.html
// it has composition {b.i text} and quotes for embedding text {a '{some link} hello world}

// see cetahe/ideas#7

/// any piece of text intended for humans to read; may be formatted (eg. messages)
/// uses my as of yet unspecced (and unnamed) text format
/// formatted inline text
// TODO: make generic over KnownTag? and string format?
// TODO: name this format. i've called it "tagged text" in some places, but the name is too generic/ambiguous
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text<'a>(Vec<Span<'a>>);

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

impl Text<'_> {
    /// get an OwnedText from this text
    pub fn to_owned(&self) -> OwnedText {
        OwnedText(self.to_static())
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|i| i.is_empty())
    }

    pub fn to_static(&self) -> Text<'static> {
        Text(self.0.iter().map(|p| p.to_static()).collect())
    }
}

impl Span<'_> {
    pub fn is_empty(&self) -> bool {
        match self {
            Span::Text(cow) => cow.is_empty(),
            Span::Tag(_) => false,
        }
    }

    pub fn to_static(&self) -> Span<'static> {
        match self {
            Span::Text(cow) => Span::Text(Cow::Owned(cow.clone().into_owned())),
            Span::Tag(tag) => Span::Tag(tag.to_static()),
        }
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

impl From<String> for Language {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Serialize for OwnedText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_ref().as_tagged_text().to_string())
    }
}

impl<'de> Deserialize<'de> for OwnedText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Str(String);
        let s = Str::deserialize(deserializer)?;
        Ok(Text::parse(&s.0).to_owned())
    }
}

#[cfg(feature = "utoipa")]
mod schema {
    use utoipa::{
        openapi::{schema::Schema, RefOr},
        schema, PartialSchema, ToSchema,
    };

    use super::OwnedText;

    impl PartialSchema for OwnedText {
        fn schema() -> RefOr<Schema> {
            let schema = schema!(String)
                .title(Some("Text"))
                .description(Some("formatted tagged text"))
                .build();
            RefOr::T(Schema::Object(schema))
        }
    }

    impl ToSchema for OwnedText {}
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

/// potential future types
#[allow(unused)]
mod next {
    use std::borrow::Cow;

    use super::tags::KnownTag;

    /// parsed text
    // TODO: generic
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Text<'a> {
        str: Cow<'a, str>,
        spans: Vec<Span>,
    }

    /// unparsed text
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TextRaw<'a> {
        str: Cow<'a, str>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Span {
        /// a raw string
        // Text(Cow<'a, str>),
        Text(usize, usize),

        /// a formatting tag
        Tag(Tag),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Tag {
        name: (usize, usize),
        params: Vec<Span>,
    }

    pub type OwnedText = Text<'static>;

    pub enum SpanRef<'a> {
        /// a raw string
        Text(Cow<'a, str>),

        /// a formatting tag
        Tag(TagRef<'a>),
    }

    pub struct TagRef<'a> {
        name: Cow<'a, str>,
        params: Vec<SpanRef<'a>>,
    }

    impl<'a> Text<'a> {
        pub fn parse(_s: &'a str) -> Text<'a> {
            todo!()
        }

        pub fn into_static(self) -> Text<'static> {
            Text {
                str: Cow::Owned(self.str.into_owned()),
                spans: self.spans.clone(),
            }
        }

        pub fn walk<W>(&self, mut w: W) -> W::Output
        where
            W: TextWalker,
        {
            for s in &self.spans {
                match s {
                    Span::Text(start, end) => w.text(&self.str[*start..*end]),
                    Span::Tag(_tag) => todo!(),
                };
            }
            w.finish()
        }
    }

    impl<'a> TextRaw<'a> {
        pub fn parse(&'a self) -> Text<'a> {
            Text::parse(&self.str)
        }

        pub fn into_static(self) -> TextRaw<'static> {
            TextRaw {
                str: Cow::Owned(self.str.into_owned()),
            }
        }
    }

    pub trait TextWalker {
        type Output;

        /// called when a text fragment is encountered
        fn text(&mut self, s: &str) -> Self::Output;

        /// called when a new tag starts
        fn tag_start(&mut self, name: &str) -> Self::Output;

        /// called when a tag parameter starts
        fn tag_param(&mut self) -> Self::Output;

        /// called after the last param in a tag
        fn tag_end(&mut self) -> Self::Output;

        /// called once the document is done being parsed
        fn finish(self) -> Self::Output;
    }

    /// text formatting tags
    // pub trait TextFormat: for<'a> TryInto<Self::Tag<'a>> {
    pub trait TextFormat {
        type Tag<'a>;
        type Error;

        fn parse_tag<'a>(tag: TagRef<'a>) -> Result<Self::Tag<'a>, Self::Error>;
    }

    /// inline formatting
    pub enum TextFormatInline {}

    impl TextFormat for TextFormatInline {
        type Tag<'a> = KnownTag<'a>;
        type Error = ();

        fn parse_tag<'a>(tag: TagRef<'a>) -> Result<Self::Tag<'a>, Self::Error> {
            super::Tag {
                name: tag.name,
                params: todo!(),
            }
            .try_into()
        }
    }
}
