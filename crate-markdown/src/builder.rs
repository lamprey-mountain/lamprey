use core::mem;
use std::fmt;

use crate::util::mentionable::Mentionable;

/// a utility to build markdown
pub struct Builder<W> {
    writer: W,
}

/// a utility to build block level markdown
pub struct BuilderBlock<'a, W> {
    writer: &'a mut W,
    format: BlockFormat,
}

/// a utility to build inline markdown text
pub struct BuilderInline<'a, W> {
    writer: &'a mut W,
    format: InlineFormat,
}

/// formatting for blocks
#[derive(Debug, Default, Clone)]
pub struct BlockFormat {
    pub blockquote: bool,
}

/// formatting for inline text
#[derive(Debug, Default, Clone)]
pub struct InlineFormat {
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
}

/// something that can be appended to a block builder
pub trait AppendableBlock<W: fmt::Write> {
    fn append(self, b: &mut BuilderBlock<'_, W>);
}

/// something that can be appended to an inline builder
pub trait AppendableInline<W: fmt::Write> {
    fn append(self, b: &mut BuilderInline<'_, W>);
}

// TODO: AppendableList, impl for Vec<_>, slice, fnmut
// list items take inline nodes

impl<W, F> AppendableInline<W> for F
where
    W: fmt::Write,
    F: FnOnce(&mut BuilderInline<'_, W>), // F: for<'b, 'w> FnOnce(&'b mut BuilderInline<'w, W>) -> &'b mut BuilderInline<'w, W>,
{
    fn append(self, b: &mut BuilderInline<'_, W>) {
        self(b);
    }
}

impl<W: fmt::Write> AppendableInline<W> for String {
    fn append(self, b: &mut BuilderInline<'_, W>) {
        b.text(&self);
    }
}

impl<W: fmt::Write> AppendableInline<W> for &str {
    fn append(self, b: &mut BuilderInline<'_, W>) {
        b.text(self);
    }
}

impl<W, F> AppendableBlock<W> for F
where
    W: fmt::Write,
    F: FnOnce(&mut BuilderBlock<'_, W>),
{
    fn append(self, b: &mut BuilderBlock<'_, W>) {
        self(b);
    }
}

impl<W: fmt::Write> AppendableBlock<W> for String {
    fn append(self, b: &mut BuilderBlock<'_, W>) {
        b.text(&self);
    }
}

impl<W: fmt::Write> AppendableBlock<W> for &str {
    fn append(self, b: &mut BuilderBlock<'_, W>) {
        b.text(self);
    }
}

impl<W: fmt::Write> Builder<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn block(&mut self) -> BuilderBlock<'_, W> {
        BuilderBlock {
            writer: &mut self.writer,
            format: Default::default(),
        }
    }

    pub fn inline(&mut self) -> BuilderInline<'_, W> {
        BuilderInline {
            writer: &mut self.writer,
            format: Default::default(),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

// TODO: h2..h6, code blocks, etc
impl<'a, W: fmt::Write> BuilderBlock<'a, W> {
    /// write a new level 1 header
    pub fn h1<A: AppendableInline<W>>(&mut self, content: A) -> &mut Self {
        self.writer.write_str("# ").unwrap();
        let mut b = BuilderInline {
            writer: &mut *self.writer,
            format: Default::default(),
        };
        content.append(&mut b);
        self.writer.write_char('\n').unwrap();
        self
    }

    /// write a new paragraph
    pub fn paragraph<A: AppendableInline<W>>(&mut self, content: A) -> &mut Self {
        let mut b = BuilderInline {
            writer: &mut *self.writer,
            format: Default::default(),
        };
        content.append(&mut b);
        // TODO: verify that this correctly formats paragraphs
        self.writer.write_char('\n').unwrap();
        self
    }

    /// insert an extra newline
    #[doc(alias = "br")]
    pub fn newline(&mut self) -> &mut Self {
        self.writer.write_char('\n').unwrap();
        self
    }

    /// write a new blockquote
    pub fn blockquote<A: AppendableBlock<W>>(&mut self, content: A) -> &mut Self {
        let was = mem::replace(&mut self.format.blockquote, true);
        content.append(self);
        self.format.blockquote = was;
        self
    }

    fn text(&mut self, text: &str) {
        if self.format.blockquote {
            for line in text.lines() {
                self.writer.write_str("> ").unwrap();
                self.writer.write_str(line).unwrap();
                self.writer.write_char('\n').unwrap();
            }
        } else {
            self.writer.write_str(text).unwrap();
        }
    }
}

impl<'a, W: fmt::Write> BuilderInline<'a, W> {
    /// create a new inline markdown builder
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            format: Default::default(),
        }
    }

    /// append raw markdown
    pub fn raw(&mut self, text: &str) -> &mut Self {
        self.writer.write_str(&text).unwrap();
        self
    }

    /// append plain text
    ///
    /// markdown is automatically escaped
    pub fn text(&mut self, text: &str) -> &mut Self {
        // TODO: alternative fn signature?
        // pub fn text<D: Display>(mut self, text: D) -> Self {
        self.writer.write_str(&escape_inline(text)).unwrap();
        self
    }

    /// append some emphasized text
    #[doc(alias = "italic")]
    pub fn emphasis<A: AppendableInline<W>>(&mut self, content: A) -> &mut Self {
        let was = mem::replace(&mut self.format.emphasis, true);
        if !was {
            self.raw("*");
        }
        content.append(self);
        if !was {
            self.raw("*");
        }
        self.format.emphasis = was;
        self
    }

    #[doc(alias = "bold")]
    pub fn strong<A: AppendableInline<W>>(&mut self, content: A) -> &mut Self {
        let was = mem::replace(&mut self.format.strong, true);
        if !was {
            self.raw("**");
        }
        content.append(self);
        if !was {
            self.raw("**");
        }
        self.format.strong = was;
        self
    }

    pub fn code<A: AppendableInline<W>>(&mut self, content: A) -> &mut Self {
        let was = mem::replace(&mut self.format.code, true);
        if !was {
            self.raw("`");
        }
        content.append(self);
        if !was {
            self.raw("`");
        }
        self.format.code = was;
        self
    }

    /// mention something
    pub fn mention<M: Mentionable>(&mut self, m: M) -> &mut Self {
        write!(self.writer, "{}", m.mention()).unwrap();
        self
    }

    // TODO: dedicated mention fns
    // fn mention_{user, role, channel, emoji, everyone}
    // TODO: custom emoji
}

/// escape inline markdown formatting for a string
pub fn escape_inline(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '*' | '_' | '`' | '[' | ']' | '\\' | '<' | '>' | '~' | '|'
        ) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

// TODO: use write!() and writeln!(); create a wrapper to make AppendableFoo impl Display or impl it directly?
// TODO: prevent newlines from being written via AppendableInline
// NOTE: how should spacing work? should i add an extra newline between h1 and p or not?
// TODO: generics are a pain, maybe i should only support String instead of any fmt::Write?

#[test]
fn test_builder() {
    let mut s = String::new();
    let mut builder = Builder::new(&mut s);
    builder
        .block()
        .h1("header")
        .paragraph("basic paragraph")
        .paragraph(|b: &mut BuilderInline<'_, _>| {
            b.text("fancy ")
                .emphasis("with emphasis")
                .text(" ")
                .strong(|b: &mut BuilderInline<'_, _>| {
                    b.text("strong and ").emphasis("emphasized");
                })
                .text(" no more formatting");
        })
        .newline()
        .blockquote("blockquote\nwith\nnewlines");
    assert_eq!(
        builder.into_inner(),
        "# header\nbasic paragraph\nfancy *with emphasis* **strong and *emphasized*** no more formatting\n\n> blockquote\n> with\n> newlines\n"
    );
}
