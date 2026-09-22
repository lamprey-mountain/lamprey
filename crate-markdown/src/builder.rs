use std::fmt;

// TODO: design this better

/// a utility to build markdown
pub struct Builder {
    // TODO
}

/// a utility to build inline markdown text
pub struct BuilderInline<W> {
    writer: W,
}

impl Builder {
    pub fn new() -> Self {
        todo!()
    }
}

impl<W: fmt::Write> BuilderInline<W> {
    /// create a new inline markdown builder
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// append plain text
    ///
    /// markdown is automatically escaped
    pub fn text(&mut self, text: &str) -> &mut Self {
        self.writer.write_str(&escape_inline(text)).unwrap();
        self
    }

    pub fn emphasis<F: FnOnce(&mut Self) -> &mut Self>(&mut self, f: F) -> &mut Self {
        // TODO: impl this
        // write *, set emphasis flag
        f(self);
        // write *, unset emphasis flag
        self
    }

    // alternative fn signature:
    // pub fn text<D: Display>(mut self, text: D) -> Self {

    // fn code
    // fn italic
    // fn bold
    // fn mention_user user_id: impl Into<UserId>
    // fn mention_{role, channel, everyone}
    // fn custom_emoji emoji: into CustomEmojiData
    // what else?

    // /// render this into a string
    // pub fn render(mut self) -> String {
    //     format!("{self}")
    // }

    // /// render this into a [`fmt::Formatter`]
    // pub fn render_to(mut self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    //     todo!()
    // }
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
