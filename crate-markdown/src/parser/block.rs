use crate::parser::ParseContext;
use crate::prelude::*;
use crate::tokenizer::TokenKind;
use crate::tree::node::{BlockKind, NodeIndex, NodeKind};
use crate::tree::Tree;

impl<'a> ParseContext<'a> {
    pub fn parse_document(mut self) -> Tree {
        let doc_span = (0, self.builder.source().len() as Len).into();
        let root = self.builder.push_node(NodeKind::Document, doc_span);

        while let Some(token) = self.tokenizer.peek() {
            let text_span = token.span;

            if let Some(ref cache) = self.cache {
                if let Some(_reused) = cache.find_reusable_block(text_span.start) {
                    // TODO: graft node and fast forward
                    // self.tokenizer.fast_forward(reused_length);
                    // continue;
                }
            }

            self.parse_block(root);
        }

        self.builder.build()
    }

    fn parse_block(&mut self, parent: NodeIndex) {
        let token = if let Some(tok) = self.tokenizer.peek() {
            tok
        } else {
            return;
        };

        match token.kind {
            TokenKind::Hash => {
                self.parse_header(parent);
            }
            TokenKind::Backtick3 => {
                self.parse_codeblock(parent);
            }
            TokenKind::AngleClose => {
                self.parse_blockquote(parent);
            }
            // TODO: list parsing, tables etc...
            _ => {
                self.parse_paragraph(parent);
            }
        }
    }

    fn parse_header(&mut self, parent: NodeIndex) {
        let start = self.tokenizer.peek().unwrap().span.start;
        let mut level = 0;

        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Hash {
                level += 1;
                self.tokenizer.advance();
            } else {
                break;
            }
        }

        // skip whitespace
        if let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Whitespace {
                self.tokenizer.advance();
            }
        }

        // read until newline
        let mut end = start;
        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Newline {
                self.tokenizer.advance();
                break;
            }
            end = tok.span.end;
            self.tokenizer.advance();
        }

        let kind = match level {
            1 => BlockKind::Header1,
            2 => BlockKind::Header2,
            3 => BlockKind::Header3,
            4 => BlockKind::Header4,
            5 => BlockKind::Header5,
            _ => BlockKind::Header6,
        };

        let node = self
            .builder
            .push_node(NodeKind::Block(kind), (start, end).into());
        self.builder.add_child(parent, node);

        self.parse_inline(node, (start, end).into());
    }

    fn parse_codeblock(&mut self, parent: NodeIndex) {
        let start_tok = self.tokenizer.advance().unwrap();
        let mut end = start_tok.span.end;

        // read until matching backticks or EOF
        while let Some(tok) = self.tokenizer.peek() {
            end = tok.span.end;
            self.tokenizer.advance();
            if tok.kind == TokenKind::Backtick3 {
                break;
            }
        }

        let node = self.builder.push_node(
            NodeKind::Block(BlockKind::Codeblock),
            (start_tok.span.start, end).into(),
        );
        self.builder.add_child(parent, node);
    }

    fn parse_blockquote(&mut self, parent: NodeIndex) {
        let start = self.tokenizer.advance().unwrap().span.start;
        let mut end = start;

        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Newline {
                self.tokenizer.advance();
                break;
            }
            end = tok.span.end;
            self.tokenizer.advance();
        }

        let node = self
            .builder
            .push_node(NodeKind::Block(BlockKind::Blockquote), (start, end).into());
        self.builder.add_child(parent, node);
        self.parse_inline(node, (start, end).into());
    }

    fn parse_paragraph(&mut self, parent: NodeIndex) {
        let start = self.tokenizer.peek().unwrap().span.start;
        let mut end = start;

        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Newline {
                let mut peek_again = self.tokenizer.clone();
                peek_again.advance();
                if let Some(next) = peek_again.peek() {
                    if next.kind == TokenKind::Newline {
                        self.tokenizer.advance();
                        break;
                    }
                } else {
                    break;
                }
            }
            end = tok.span.end;
            self.tokenizer.advance();
        }

        let node = self
            .builder
            .push_node(NodeKind::Block(BlockKind::Paragraph), (start, end).into());
        self.builder.add_child(parent, node);
        self.parse_inline(node, (start, end).into());
    }
}
