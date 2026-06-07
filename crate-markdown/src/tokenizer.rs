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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
#[repr(u8)]
pub enum TokenKind {
    /// uuid pattern for mentions and emoji
    #[regex("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")]
    Uuid,

    /// url pattern for autolinks and link destinations
    #[regex(r"https?://[^\s\]\)>]+")]
    Url,

    #[regex(r"[ \t]+")]
    Whitespace,

    #[regex(r"\n+")]
    Newline,

    #[token("\\")]
    Escape,

    #[logos(text("***"))]
    Asterisk3,

    #[logos(text("**"))]
    Asterisk2,

    #[logos(text("*"))]
    Asterisk1,

    /// any text that didn't match the above
    #[regex(".+?", priority = 1)]
    Text,
}

impl<'s> Tokenizer<'s> {
    pub fn new(source: &'s str) -> Self {
        Self {
            source,
            lexer: TokenKind::lexer(source),
        }
    }

    pub fn advance(&mut self) -> Option<Token> {
        todo!()
    }
}
