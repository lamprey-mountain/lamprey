//! code for parsing tagged text

use std::{borrow::Cow, iter::Peekable, ops::Range};

use logos::{Lexer, Logos};

use super::{Span, Tag, Text};

// TODO: stricter parsing mode for linting?
// TODO: return Err instead of panicking
const STRICT: bool = false;

#[derive(Debug, Logos)]
enum Token {
    #[token("{", priority = 5)]
    Open,
    #[token("}", priority = 5)]
    Close,
    #[token("~", priority = 5)]
    Tag,
    #[regex("[^{}~]+", priority = 4, callback = |lex| lex.span())]
    Text(Range<usize>),
}

/// helper to parse
struct Parser<'a, 'src> {
    lex: Peekable<&'a mut Lexer<'src, Token>>,
}

#[derive(Debug, Clone)]
enum ParseSpan<'a> {
    Text(&'a str),
    Token(&'a str, Vec<Text<'a>>),
    EndOfAttr,
    Eof,
}

#[derive(Debug, Clone)]
enum ParseRes<'a> {
    EndOfAttr(Vec<Span<'a>>),
    EndOfInput(Vec<Span<'a>>),
}

impl<'a, 'src> Parser<'a, 'src> {
    pub fn new(lex: &'a mut Lexer<'src, Token>) -> Self {
        Self {
            lex: lex.by_ref().peekable(),
        }
    }
}

fn parse<'b>(a: &mut Parser, s: &'b str) -> ParseRes<'b> {
    let mut parts = vec![];
    loop {
        match parse_text(a, s) {
            ParseSpan::Text(s) => parts.push(Span::Text(Cow::Borrowed(s))),
            ParseSpan::Eof => break,
            ParseSpan::EndOfAttr => return ParseRes::EndOfAttr(parts),
            ParseSpan::Token(name, params) => parts.push(Span::Tag(Tag {
                name: Cow::Borrowed(name),
                params,
            })),
        };
    }
    ParseRes::EndOfInput(parts)
}

fn parse_attr<'b>(a: &mut Parser, s: &'b str) -> Option<Text<'b>> {
    match a.lex.peek() {
        Some(t) => match t.as_ref().unwrap() {
            Token::Open => {
                a.lex.next();
                match parse(a, s) {
                    ParseRes::EndOfAttr(vec) => Some(Text(vec)),
                    ParseRes::EndOfInput(vec) => {
                        if STRICT {
                            panic!("missing closing braces");
                        } else {
                            // automatically close ending braces
                            Some(Text(vec))
                        }
                    }
                }
            }
            _ => None,
        },
        None => None,
    }
}

fn parse_text<'b>(a: &mut Parser, s: &'b str) -> ParseSpan<'b> {
    match a.lex.next() {
        Some(t) => match t.unwrap() {
            Token::Open => {
                if STRICT {
                    panic!("unexpected open bracket")
                } else {
                    // i'll assume
                    ParseSpan::Text("{")
                }
            }
            Token::Close => ParseSpan::EndOfAttr,
            Token::Tag => match a.lex.next() {
                Some(t) => match t.unwrap() {
                    Token::Text(r) => {
                        let name = &s[r];
                        let mut attrs = vec![];
                        while let Some(attr) = parse_attr(a, s) {
                            attrs.push(attr);
                        }
                        ParseSpan::Token(name, attrs)
                    }

                    // ~ before special {}~ acts like an escape
                    Token::Open => ParseSpan::Text("{"),
                    Token::Close => ParseSpan::Text("}"),
                    Token::Tag => ParseSpan::Text("~"),
                },
                None => ParseSpan::Eof,
            },
            Token::Text(r) => ParseSpan::Text(&s[r]),
        },
        None => ParseSpan::Eof,
    }
}

impl<'a> Text<'a> {
    /// parse a str
    ///
    /// parsing is designed to never fail
    pub fn parse(s: &'a str) -> Text<'a> {
        let mut lex = Token::lexer(s);
        let mut parser = Parser::new(&mut lex);
        let spans = match parse(&mut parser, s) {
            ParseRes::EndOfAttr(mut vec) => {
                if STRICT {
                    panic!("unexpected eof")
                } else {
                    // its probably better to send the raw text than to truncate
                    // maybe i could start parsing again, but if the text is broken then dont bother
                    vec.push(Span::Text(Cow::Borrowed(lex.remainder())));
                    vec
                }
            }
            ParseRes::EndOfInput(vec) => vec,
        };
        Self(spans)
    }
}
