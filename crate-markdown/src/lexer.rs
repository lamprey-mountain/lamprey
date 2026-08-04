//! splits input text into tokens

use logos::{Lexer as LogosLexer, Logos};

use crate::prelude::*;

// TODO: rename module and types to lexer
#[derive(Clone)]
pub struct Lexer<'source> {
    source: &'source str,
    lexer: LogosLexer<'source, TokenKind>,
    offset: usize,
    peeked: Option<Token>,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

// TODO: verify that everything i need is here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
#[rustfmt::skip]
pub enum TokenKind {
    // basic symbols
    #[token("~~")]        Tilde2,
    #[token("|")]         Pipe,
    #[token("||")]        Pipe2,
    #[token("@")]         At,
    #[token("&")]         Ampersand,
    #[token("#")]         Hash,
    #[token(":")]         Colon,
    #[token("-")]         Dash,
    #[token(".")]         Dot,
    #[token("[")]         BracketOpen,
    #[token("]")]         BracketClose,
    #[token("(")]         ParenOpen,
    #[token(")")]         ParenClose,
    #[token("<")]         AngleOpen,
    #[token(">")]         AngleClose,
    #[regex(r"\n")]       Newline,
    #[token(r"\")]        Backslash,
    #[token("***")] Asterisk3,
    #[token("**")]  Asterisk2,
    #[token("*")]   Asterisk1,

    #[regex("`+", |lex| lex.slice().len() as u16)]
    Backticks(u16),

    #[regex(r"[\p{Emoji_Presentation}\u{200d}\u{fe0f}]+", priority = 3)]
    UnicodeEmoji,

    /// uuid pattern, used for mentions and emoji
    #[regex("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")]
    Uuid,

    // TODO: handle nested parentheses
    /// url pattern for autolinks and link destinations
    #[regex(r"https?://[^\s\]\)>]+")]
    Url,

    #[regex("[0-9]+")]
    Number,

    /// any text that didn't match the above
    #[regex(r"[^ \t\n*\\`<>\[\]\(\)#@:~.\-&|0-9][^ \t\n*\\`<>\[\]\(\)#@:~.\-&|]*")]
    Text,

    // part of Text?
    #[regex(r"[ \t]+")]
    Whitespace,

    // ???
    Error,
}

impl<'s> Lexer<'s> {
    pub fn new(source: &'s str) -> Self {
        Self {
            source,
            lexer: TokenKind::lexer(source),
            offset: 0,
            peeked: None,
        }
    }

    pub fn peek(&mut self) -> Option<Token> {
        if self.peeked.is_none() {
            self.peeked = self.next_token();
        }
        self.peeked.clone()
    }

    pub fn peek_kind(&mut self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    pub fn advance(&mut self) -> Option<Token> {
        if let Some(token) = self.peeked.take() {
            Some(token)
        } else {
            self.next_token()
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        let first = self.lexer.next()?;
        let mut kind = first.unwrap_or(TokenKind::Error);
        let mut span = self.lexer.span();

        loop {
            let mut lookahead = self.lexer.clone();
            match lookahead.next() {
                Some(Ok(next_kind)) => match merged_kind(kind, next_kind) {
                    Some(k) => {
                        span.end = lookahead.span().end;
                        self.lexer = lookahead;
                        kind = k;
                    }
                    None => break,
                },
                _ => break,
            }
        }

        Some(Token {
            kind,
            span: Span::from(span) + self.offset as Len,
        })
    }

    pub fn text(&self, span: Span) -> &str {
        &self.source[span.start as usize..span.end as usize]
    }

    // TODO: remove? idk if i need this for incremental reparsing, maybe its good enough to tokenize the whole document on every edit?
    #[cfg(any())]
    pub fn fast_forward(&mut self, bytes: usize) {
        self.offset += bytes;
        if self.offset < self.source.len() {
            // FIXME: lexing part of a token (eg. fast forwarding to "*" inside "**", should still be Asterisk2 not Asterisk1)
            self.lexer = TokenKind::lexer(&self.source[self.offset..]);
        } else {
            self.lexer = TokenKind::lexer("");
        }
    }

    /// skip over as much whitespace as possible
    ///
    /// returns the number of whitespace tokens skipped
    // WARNING: don't use this outside of peeking since the parser needs to create text nodes for every piece of text!
    // TODO: maybe remove this, it could be a footgun
    pub fn consume_whitespace(&mut self) -> usize {
        let mut consumed = 0;
        while self.peek_kind() == Some(TokenKind::Whitespace) {
            self.advance();
            consumed += 1;
        }
        consumed
    }
}

/// if two tokens can be merged, returns the kind of the merged token
fn merged_kind(a: TokenKind, b: TokenKind) -> Option<TokenKind> {
    match (a, b) {
        // TODO: correctly merge Text and Whitespace? this seems to be trickier than it looks though
        // eg. in headers, whitespace between the hashes and text gets trimmed. this doesn't work if i naively allow merging whitespace and text.
        (TokenKind::Whitespace, TokenKind::Whitespace) => Some(TokenKind::Whitespace),
        (TokenKind::Text, TokenKind::Text) => Some(TokenKind::Text),
        _ => None,
    }
}

// PERF: maybe use ropes or something that handles edits better
pub struct Source(pub(crate) String);

impl Source {
    /// apply an edit
    pub fn new(source: &str) -> Self {
        Self(source.to_string())
    }

    /// apply an edit
    pub fn edit(&mut self, delete: Span, insert: &str) {
        self.0.replace_range(
            delete.start as usize..(delete.end as usize).min(self.0.len()),
            insert,
        );
    }
}

// impl logos::Source for Source {
//     type Slice<'a>
//     where
//         Self: 'a;

//     fn len(&self) -> usize {
//         todo!()
//     }

//     fn read<'a, Chunk>(&'a self, offset: usize) -> Option<logos::source::Chunk>
//     where
//         logos::source::Chunk: logos::source::Chunk<'a>,
//     {
//         todo!()
//     }

//     fn slice(&self, range: std::ops::Range<usize>) -> Option<Self::Slice<'_>> {
//         todo!()
//     }

//     unsafe fn slice_unchecked(&self, range: std::ops::Range<usize>) -> Self::Slice<'_> {
//         todo!()
//     }

//     fn is_boundary(&self, index: usize) -> bool {
//         todo!()
//     }
// }
