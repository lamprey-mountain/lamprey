//! code for rendering tagged text

use std::fmt::Display;

use super::{tags::KnownTag, Span, Tag, Text};

/// a struct whos Display impl outputs html
pub struct HtmlFormatter<'a>(&'a Text<'a>);

struct HtmlSanitized<'a>(&'a str);

struct HtmlFormatterInner<'a>(&'a Span<'a>);

/// a struct whos Display impl outputs plain text
pub struct PlainFormatter<'a>(&'a Text<'a>);

struct PlainFormatterInner<'a>(&'a Span<'a>);

/// a struct whos Display impl outputs tagged text (the native wire format)
pub struct TaggedTextFormatter<'a>(&'a Text<'a>);

struct TaggedTextFormatterInner<'a>(&'a Span<'a>);

impl Text<'_> {
    pub fn as_html(&self) -> HtmlFormatter {
        HtmlFormatter(self)
    }

    pub fn as_plain(&self) -> PlainFormatter {
        PlainFormatter(self)
    }

    pub fn as_tagged_text(&self) -> TaggedTextFormatter {
        TaggedTextFormatter(self)
    }
}

impl Display for HtmlFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in &self.0 .0 {
            write!(f, "{}", HtmlFormatterInner(i))?;
        }
        Ok(())
    }
}

impl Display for PlainFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in &self.0 .0 {
            write!(f, "{}", PlainFormatterInner(i))?;
        }
        Ok(())
    }
}

impl Display for TaggedTextFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in &self.0 .0 {
            write!(f, "{}", TaggedTextFormatterInner(i))?;
        }
        Ok(())
    }
}

impl Display for HtmlSanitized<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // kind of ugly but it works
        let s = self
            .0
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;");
        write!(f, "{s}")?;
        Ok(())
    }
}

impl Display for HtmlFormatterInner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Span::Text(t) => write!(f, "{}", HtmlSanitized(t))?,
            Span::Tag(tag) => {
                let known: KnownTag = tag.clone().try_into().unwrap();
                match known {
                    KnownTag::Bold(text) => write!(f, "<b>{}</b>", HtmlFormatter(&text))?,
                    KnownTag::Emphasis(text) => write!(f, "<em>{}</em>", HtmlFormatter(&text))?,
                    KnownTag::Sub(text) => write!(f, "<sub>{}</sub>", HtmlFormatter(&text))?,
                    KnownTag::Sup(text) => write!(f, "<sup>{}</sup>", HtmlFormatter(&text))?,
                    KnownTag::Strikethrough(text) => write!(f, "<s>{}</s>", HtmlFormatter(&text))?,
                    KnownTag::Link(url, Some(text)) => {
                        write!(f, "<a href=\"{url}\">{}</a>", HtmlFormatter(&text))?
                    }
                    KnownTag::Link(url, None) => write!(f, "<a href=\"{url}\">{url}</a>")?,
                    KnownTag::Code(text, _lang) => {
                        write!(f, "<code>{}</code>", HtmlFormatter(&text))?
                    }
                    _ => todo!(),
                }
            }
        }
        Ok(())
    }
}

impl Display for PlainFormatterInner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Span::Text(t) => write!(f, "{t}")?,
            Span::Tag(tag) => {
                let known: Result<KnownTag, _> = tag.clone().try_into();
                if let Ok(KnownTag::Link(link, text)) = dbg!(known) {
                    if let Some(text) = text {
                        write!(f, "{} ({})", PlainFormatter(&text), link)?;
                    } else {
                        write!(f, "{}", link)?;
                    }
                } else {
                    let Tag { name: _, params } = tag;
                    for param in params {
                        for span in &param.0 {
                            write!(f, "{}", PlainFormatterInner(span))?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Display for TaggedTextFormatterInner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Span::Text(t) => write!(f, "{t}")?,
            Span::Tag(Tag { name, params }) => {
                write!(f, "~{name}")?;
                for param in params {
                    write!(f, "{{")?;
                    for span in &param.0 {
                        write!(f, "{}", TaggedTextFormatterInner(span))?;
                    }
                    write!(f, "}}")?;
                }
            }
        }
        Ok(())
    }
}
