use crate::parser::ParseContext;
use crate::prelude::*;
use crate::tokenizer::{TokenKind, Tokenizer};
use crate::tree::node::{InlineKind, NodeIndex, NodeKind, TextKind};

impl<'a> ParseContext<'a> {
    pub fn parse_inline(&mut self, parent: NodeIndex, span: Span) {
        // PERF: don't clone string
        // PERF: don't create a new lexer
        let source = self.builder.source().to_string();
        let text = &source[span.start as usize..span.end as usize];
        let mut inline_lexer = Tokenizer::new(text);

        while let Some(tok) = inline_lexer.advance() {
            let offset_start = tok.span.start + span.start;
            let offset_end = tok.span.end + span.start;
            let offset_span = Span {
                start: offset_start,
                end: offset_end,
            };

            match tok.kind {
                TokenKind::Asterisk1 | TokenKind::Asterisk2 | TokenKind::Asterisk3 => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Inline(InlineKind::Emphasis), offset_span);
                    self.builder.add_child(parent, node);
                }
                TokenKind::Tilde2 => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Inline(InlineKind::Strikethrough), offset_span);
                    self.builder.add_child(parent, node);
                }
                TokenKind::Pipe2 => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Inline(InlineKind::Spoiler), offset_span);
                    self.builder.add_child(parent, node);
                }
                TokenKind::Backtick => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Inline(InlineKind::Code), offset_span);
                    self.builder.add_child(parent, node);
                }
                TokenKind::BracketOpen => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Inline(InlineKind::Link), offset_span);
                    self.builder.add_child(parent, node);
                }
                TokenKind::Url => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Text(TextKind::Url), offset_span);
                    self.builder.add_child(parent, node);
                }
                TokenKind::AngleOpen => {
                    // check for mentions
                    if let Some(next) = inline_lexer.peek() {
                        if matches!(
                            next.kind,
                            TokenKind::At | TokenKind::Hash | TokenKind::Ampersand
                        ) {
                            let node = self
                                .builder
                                .push_node(NodeKind::Text(TextKind::Mention), offset_span);
                            self.builder.add_child(parent, node);
                            continue;
                        }
                    }

                    // TODO: check for emoji
                    // FIXME: merge Text nodes - sometimes the parser returns multiple fragmented Text nodes instead of one with all the text
                    let node = self
                        .builder
                        .push_node(NodeKind::Text(TextKind::Text), offset_span);
                    self.builder.add_child(parent, node);
                }
                _ => {
                    let node = self
                        .builder
                        .push_node(NodeKind::Text(TextKind::Text), offset_span);
                    self.builder.add_child(parent, node);
                }
            }
        }
    }
}
