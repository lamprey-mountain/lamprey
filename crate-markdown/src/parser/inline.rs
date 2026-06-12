use crate::lexer::Token;
use crate::parser::ParseContext;
use crate::prelude::*;

impl<'a> ParseContext<'a> {
    /// parse inline markdown
    ///
    /// the provided `stop` function can return `true` to stop inline parsing
    // PERF: is creating/nesting lots of functions ok or will it cause problems?
    pub(crate) fn parse_inline(&mut self, stop: &dyn Fn(&Token) -> bool) {
        while let Some(tok) = self.tokenizer.peek() {
            if stop(&tok) {
                break;
            }

            self.tokenizer.advance();

            match tok.kind {
                // parse a codeblock
                TokenKind::Backticks(n) => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Code).into());
                    self.builder.token(
                        NodeKind::Text(TextKind::Syntax).into(),
                        self.tokenizer.text(tok.span),
                    );
                    self.parse_inline(&|t| {
                        if let TokenKind::Backticks(m) = t.kind {
                            m == n || stop(t)
                        } else {
                            stop(t)
                        }
                    });
                    if let Some(next_tok) = self.tokenizer.peek() {
                        if let TokenKind::Backticks(m) = next_tok.kind {
                            if m == n {
                                self.tokenizer.advance();
                                self.builder.token(
                                    NodeKind::Text(TextKind::Syntax).into(),
                                    self.tokenizer.text(next_tok.span),
                                );
                            }
                        }
                    }
                    self.builder.finish_node();
                }

                // parse strong/emphasis
                TokenKind::Asterisk1 => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Emphasis).into());
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), "*");
                    self.parse_inline(&|t| t.kind == TokenKind::Asterisk1 || stop(t));
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Asterisk1 {
                            self.tokenizer.advance();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "*");
                        }
                    }
                    self.builder.finish_node();
                }
                TokenKind::Asterisk2 => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Strong).into());
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), "**");
                    self.parse_inline(&|t| t.kind == TokenKind::Asterisk2 || stop(t));
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Asterisk2 {
                            self.tokenizer.advance();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "**");
                        }
                    }
                    self.builder.finish_node();
                }
                TokenKind::Asterisk3 => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Strong).into());
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Emphasis).into());
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), "***");
                    self.parse_inline(&|t| t.kind == TokenKind::Asterisk3 || stop(t));
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Asterisk3 {
                            self.tokenizer.advance();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "***");
                        }
                    }
                    self.builder.finish_node();
                    self.builder.finish_node();
                }

                // handle escape
                TokenKind::Backslash => {
                    if let Some(next) = self.tokenizer.advance() {
                        // backslash is syntax, escaped char is text
                        let text = self.tokenizer.text(next.span).to_string();
                        self.builder
                            .token(NodeKind::Text(TextKind::Syntax).into(), "\\");
                        self.builder
                            .token(NodeKind::Text(TextKind::Text).into(), &text);
                    } else {
                        // if theres nothing to escape, the backslash becomes text
                        let text = self.tokenizer.text(tok.span).to_string();
                        self.builder
                            .token(NodeKind::Text(TextKind::Text).into(), &text);
                    };
                }

                // strikethrough
                TokenKind::Tilde2 => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Strikethrough).into());
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), "~~");
                    self.parse_inline(&|t| t.kind == TokenKind::Tilde2 || stop(t));
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Tilde2 {
                            self.tokenizer.advance();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "~~");
                        }
                    }
                    self.builder.finish_node();
                }

                // spoiler
                TokenKind::Pipe2 => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Spoiler).into());
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), "||");
                    self.parse_inline(&|t| t.kind == TokenKind::Pipe2 || stop(t));
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Pipe2 {
                            self.tokenizer.advance();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "||");
                        }
                    }
                    self.builder.finish_node();
                }

                // Url link (automatic)
                TokenKind::Url => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Autolink).into());
                    self.builder.token(
                        NodeKind::Text(TextKind::Text).into(),
                        self.tokenizer.text(tok.span),
                    );
                    self.builder.finish_node();
                }

                // link
                TokenKind::BracketOpen => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Link).into());
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), "[");
                    self.parse_inline(&|t| t.kind == TokenKind::BracketClose || stop(t));
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::BracketClose {
                            self.tokenizer.advance();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "]");

                            // check for (url)
                            if let Some(tok) = self.tokenizer.peek() {
                                if tok.kind == TokenKind::ParenOpen {
                                    self.tokenizer.advance();
                                    self.builder
                                        .token(NodeKind::Text(TextKind::Syntax).into(), "(");

                                    while let Some(nt) = self.tokenizer.peek() {
                                        if nt.kind == TokenKind::ParenClose || stop(&nt) {
                                            break;
                                        }
                                        self.tokenizer.advance();
                                        let kind = match nt.kind {
                                            TokenKind::Url => TextKind::LinkUrl,
                                            // TODO: remove TextKind::Syntax logic?
                                            // TokenKind::BracketClose | TokenKind::ParenOpen => {
                                            //     TextKind::Syntax
                                            // }
                                            // handle whitespace
                                            _ => TextKind::Text,
                                        };
                                        self.builder.token(
                                            NodeKind::Text(kind).into(),
                                            self.tokenizer.text(nt.span),
                                        );
                                    }

                                    if let Some(tok) = self.tokenizer.peek() {
                                        if tok.kind == TokenKind::ParenClose {
                                            self.tokenizer.advance();
                                            self.builder.token(
                                                NodeKind::Text(TextKind::Syntax).into(),
                                                ")",
                                            );
                                        }
                                    }
                                } else {
                                    // TODO: handle syntax error?
                                }
                            }
                        }
                    }
                    self.builder.finish_node();
                }

                // link with angle brackets, mention, or custom emoji
                TokenKind::AngleOpen => {
                    let mut temp = self.tokenizer.clone();
                    let mut tokens = Vec::new();
                    let mut close_token = None;

                    // read until the closing angle bracket
                    while let Some(nt) = temp.advance() {
                        if nt.kind == TokenKind::AngleClose {
                            close_token = Some(nt);
                            break;
                        }
                        if nt.kind == TokenKind::Whitespace
                            || nt.kind == TokenKind::Newline
                            || stop(&nt)
                        {
                            break;
                        }
                        tokens.push(nt);
                    }

                    if let Some(close) = close_token {
                        if self.is_mention(&tokens) {
                            let span = Span {
                                start: tok.span.start,
                                end: close.span.end,
                            };
                            let text = self.tokenizer.text(span);
                            self.builder
                                .token(NodeKind::Text(TextKind::Mention).into(), text);
                            for _ in 0..tokens.len() + 1 {
                                self.tokenizer.advance();
                            }
                        } else if self.is_emoji(&tokens) {
                            let span = Span {
                                start: tok.span.start,
                                end: close.span.end,
                            };
                            let text = self.tokenizer.text(span);
                            self.builder
                                .token(NodeKind::Text(TextKind::CustomEmoji).into(), text);
                            for _ in 0..tokens.len() + 1 {
                                self.tokenizer.advance();
                            }
                        } else if self.is_url(&tokens) {
                            self.builder
                                .start_node(NodeKind::Inline(InlineKind::Autolink).into());
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), "<");
                            let t = self.tokenizer.advance().expect("token exists");
                            self.builder.token(
                                NodeKind::Text(TextKind::Text).into(),
                                self.tokenizer.text(t.span),
                            );
                            self.tokenizer.advance(); // consume >
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), ">");
                            self.builder.finish_node();
                        } else {
                            self.builder
                                .token(NodeKind::Text(TextKind::Text).into(), "<");
                        }
                    } else {
                        self.builder
                            .token(NodeKind::Text(TextKind::Text).into(), "<");
                    }
                }

                // otherwise try to parse the token as text
                _ => {
                    let text = self.tokenizer.text(tok.span).to_string();
                    // FIXME: use correct TextKind instead of always TextKind::Text
                    self.builder
                        .token(NodeKind::Text(TextKind::Text).into(), &text);
                }
            }
        }
    }

    fn is_mention(&self, tokens: &[Token]) -> bool {
        match tokens {
            [t1, t2] => match t1.kind {
                TokenKind::At | TokenKind::Ampersand | TokenKind::Hash => {
                    t2.kind == TokenKind::Uuid
                        || (t1.kind == TokenKind::At && self.tokenizer.text(t2.span) == "everyone")
                }
                _ => false,
            },
            _ => false,
        }
    }

    // TODO: macros for matching token patterns?
    fn is_emoji(&self, tokens: &[Token]) -> bool {
        match tokens {
            [Token {
                kind: TokenKind::Colon,
                ..
            }, Token {
                kind: TokenKind::Text,
                ..
            }, Token {
                kind: TokenKind::Colon,
                ..
            }, Token {
                kind: TokenKind::Uuid,
                ..
            }] => true,
            [t1, Token {
                kind: TokenKind::Colon,
                ..
            }, Token {
                kind: TokenKind::Text,
                ..
            }, Token {
                kind: TokenKind::Colon,
                ..
            }, Token {
                kind: TokenKind::Uuid,
                ..
            }] if t1.kind == TokenKind::Text && self.tokenizer.text(t1.span) == "a" => true,
            _ => false,
        }
    }

    fn is_url(&self, tokens: &[Token]) -> bool {
        matches!(
            tokens,
            [Token {
                kind: TokenKind::Url,
                ..
            }]
        )
    }
}
