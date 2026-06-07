//! splits input text into tokens

use logos::{Lexer, Logos};

use crate::prelude::*;

pub struct Tokenizer<'source> {
    source: &'source str,
    lexer: Lexer<'source, TokenKind>,
}

pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

// TODO: verify that everything i need is here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
#[repr(u8)]
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
    #[token("[")]         BracketOpen,
    #[token("]")]         BracketClose,
    #[token("(")]         ParenOpen,
    #[token(")")]         ParenClose,
    #[regex(r"\n")]       Newline,
    #[token(r"\")]        Backslash,
    #[logos(text("***"))] Asterisk3,
    #[logos(text("**"))]  Asterisk2,
    #[logos(text("*"))]   Asterisk1,

    /// uuid pattern, used for mentions and emoji
    #[regex("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")]
    Uuid,

    // TODO: redo url tokenization to handle urls with paranthases properly:
    // [foo](https://en.wikipedia.org/wiki/Backrooms_(film))
    // [foo](https://en.wikipedia.org/wiki/Science_fiction_film)
    /// url pattern for autolinks and link destinations
    #[regex(r"https?://[^\s\]\)>]+")]
    Url,

    /// any text that didn't match the above
    // TODO: verify this is correct
    #[regex(r"[^ \t\n*\\`<>\[\]\(\)#@:~.\-&|]+")]
    // #[regex(".+?", priority = 2)]
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
        }
    }

    pub fn advance(&mut self) -> Option<Token> {
        self.lexer.next().map(|kind| {
            let s = self.lexer.span();
            Token {
                kind: kind.unwrap_or(TokenKind::Error),
                span: Span {
                    start: s.start as Len,
                    end: s.end as Len,
                },
            }
        })
    }
}
