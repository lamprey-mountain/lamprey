//! splits input text into tokens

use logos::{Lexer, Logos};

use crate::prelude::*;

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
    #[token(".")]         Dot,
    #[token("[")]         BracketOpen,
    #[token("]")]         BracketClose,
    #[token("(")]         ParenOpen,
    #[token(")")]         ParenClose,
    #[token("<")]         AngleOpen,
    #[token(">")]         AngleClose,
    #[logos(text("```"))] Backtick3,
    #[token("`")]         Backtick,
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
