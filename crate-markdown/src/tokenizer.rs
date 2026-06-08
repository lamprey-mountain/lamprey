//! splits input text into tokens

use logos::{Lexer, Logos};

use crate::prelude::*;

// TODO: rename module and types to lexer
#[derive(Clone)]
pub struct Tokenizer<'source> {
    source: &'source str,
    lexer: Lexer<'source, TokenKind>,
    offset: usize,
    peeked: Option<Token>,
}

#[derive(Clone)]
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
    #[logos(text("```"))] Backtick3,
    #[token("`")]         Backtick,
    // #[token("`+")]        Backtick(u16), // TODO: use this instead of Backtick/Backtick3?
    #[regex(r"\n")]       Newline,
    #[token(r"\")]        Backslash,
    #[logos(text("***"))] Asterisk3,
    #[logos(text("**"))]  Asterisk2,
    #[logos(text("*"))]   Asterisk1,

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
    #[regex(r"[^ \t\n*\\`<>\[\]\(\)#@:~.\-&|0-9]+")]
    Text,

    // part of Text?
    #[regex(r"[ \t]+")]
    Whitespace,

    // ???
    Error,
}

impl<'s> Tokenizer<'s> {
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

    pub fn advance(&mut self) -> Option<Token> {
        if let Some(token) = self.peeked.take() {
            Some(token)
        } else {
            self.next_token()
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.lexer.next().map(|kind| {
            let s = self.lexer.span();
            Token {
                kind: kind.unwrap_or(TokenKind::Error),
                span: ((s.start + self.offset) as Len, (s.end + self.offset) as Len).into(),
            }
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
        self.0
            .replace_range(delete.start as usize..delete.end as usize, insert);
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
