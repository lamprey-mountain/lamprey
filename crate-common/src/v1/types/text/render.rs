//! code for rendering tagged text

use std::fmt::Display;

use super::{
    tags::{BlockInner, Document, KnownTag},
    Span, Tag, Text,
};

/// a struct whose Display impl outputs html
pub struct HtmlFormatter<'a>(&'a Text<'a>);

pub struct HtmlDocumentFormatter<'a>(&'a Document);

/// sanitizes text in its display impl to prevent accidental html formatting
struct HtmlSanitized<'a>(&'a str);

struct HtmlFormatterInner<'a>(&'a Span<'a>);

/// a struct whose Display impl outputs plain text
pub struct PlainFormatter<'a>(&'a Text<'a>);

struct PlainFormatterInner<'a>(&'a Span<'a>);

/// a struct whose Display impl outputs tagged text (the native wire format)
pub struct TaggedTextFormatter<'a>(&'a Text<'a>);

struct TaggedTextFormatterInner<'a>(&'a Span<'a>);

/// a struct whose Display impl outputs tagged text (the native wire format)
pub struct MarkdownFormatter<'a>(&'a Text<'a>);

/// sanitizes text in its display impl to prevent accidental markdown formatting
struct MarkdownSanitized<'a>(&'a str);

struct MarkdownFormatterInner<'a>(&'a Span<'a>);

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

    pub fn as_markdown(&self) -> MarkdownFormatter {
        MarkdownFormatter(self)
    }
}

impl Document {
    pub fn as_html(&self) -> HtmlDocumentFormatter {
        HtmlDocumentFormatter(self)
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

impl Display for HtmlDocumentFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in &self.0 .0 {
            match &i.0 {
                BlockInner::Paragraph { text } => write!(f, "<p>{}</p>", text.0.as_html())?,
                BlockInner::Heading { text, level } => {
                    write!(f, "<h{level}>{}</h{level}>", text.0.as_html())?
                }
                BlockInner::Blockquote { text } => {
                    write!(f, "<blockquote>{}</blockquote>", text.as_html())?
                }
                BlockInner::Code { lang, text } => match lang {
                    Some(lang) => write!(
                        f,
                        r#"<pre><code lang="{}">{}</code></pre>"#,
                        lang.0,
                        text.0.as_html()
                    )?,
                    None => write!(f, "<pre>{}</pre>", text.0.as_html())?,
                },
                BlockInner::ListUnordered { items } => {
                    write!(f, "<ul>")?;
                    for i in items {
                        write!(f, "<li>{}</li>", i.as_html())?
                    }
                    write!(f, "</ul>")?;
                }
                BlockInner::ListOrdered { items } => {
                    write!(f, "<ol>")?;
                    for i in items {
                        write!(f, "<li>{}</li>", i.as_html())?
                    }
                    write!(f, "</ol>")?;
                }
                _ => todo!(),
            }
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

impl Display for MarkdownFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in &self.0 .0 {
            write!(f, "{}", MarkdownFormatterInner(i))?;
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

impl Display for MarkdownSanitized<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // kind of ugly but it works
        // TODO: use aho corasick
        // TODO: escape less aggressively
        let s = self
            .0
            .replace("\\", "\\\\")
            .replace("*", "\\*")
            .replace("_", "\\_")
            .replace("`", "\\`")
            .replace("[", "\\[")
            .replace("]", "\\]")
            .replace("<", "\\<")
            .replace(">", "\\>")
            .replace("+", "\\+")
            .replace("!", "\\!")
            .replace(".", "\\.")
            .replace("|", "\\|");
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
                    #[cfg(feature = "formatting_extra")]
                    KnownTag::Sub(text) => write!(f, "<sub>{}</sub>", HtmlFormatter(&text))?,
                    #[cfg(feature = "formatting_extra")]
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
                if let Ok(KnownTag::Link(link, text)) = known {
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

impl Display for MarkdownFormatterInner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Span::Text(t) => write!(f, "{}", MarkdownSanitized(t))?,
            Span::Tag(tag) => {
                let known: KnownTag = tag.clone().try_into().unwrap();
                match known {
                    KnownTag::Bold(text) => write!(f, "**{}**", MarkdownFormatter(&text))?,
                    KnownTag::Emphasis(text) => write!(f, "*{}*", MarkdownFormatter(&text))?,
                    KnownTag::Strikethrough(text) => write!(f, "~~{}~~", MarkdownFormatter(&text))?,
                    KnownTag::Link(url, Some(text)) => {
                        write!(f, "[{}]({url})", MarkdownFormatter(&text))?
                    }
                    KnownTag::Link(url, None) => write!(f, "[{url}]({url})")?,
                    KnownTag::Code(text, _lang) => write!(f, "`{}`", MarkdownFormatter(&text))?,
                    #[cfg(feature = "formatting_extra")]
                    KnownTag::Spoiler(text, None) => write!(f, "||{}||", MarkdownFormatter(&text))?,
                    #[cfg(feature = "formatting_extra")]
                    KnownTag::Spoiler(text, Some(why)) => write!(
                        f,
                        "||{}|| ({})",
                        MarkdownFormatter(&text),
                        MarkdownSanitized(&why)
                    )?,
                    #[cfg(feature = "formatting_extra")]
                    KnownTag::Math(m) => write!(f, "`{}`", MarkdownSanitized(m))?,
                    _ => todo!(),
                }
            }
        }
        Ok(())
    }
}
