use crate::parser::ParseContext;
use crate::prelude::*;
use crate::tokenizer::Token;

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
                // FIXME: handle n (what happens if there is more than 1 backtick?)
                TokenKind::Backticks(_) => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Code).into());
                    // FIXME: handle closing backtick
                    self.parse_inline(stop);
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

                // TODO: finish implementing other inline tokens
                // Tilde2 strikethrough
                // Pipe2 spoiler
                // Url link (automatic)
                // BracketOpen link
                // AngleOpen link

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
}
