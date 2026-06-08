use crate::parser::ParseContext;
use crate::prelude::*;

impl<'a> ParseContext<'a> {
    pub(crate) fn parse_document(mut self) -> Tree {
        self.builder.start_node(NodeKind::Document.into());

        // keep parsing blocks until we run out of tokens
        while let Some(_token) = self.tokenizer.peek() {
            self.parse_block();
        }

        self.builder.finish_node();
        Tree {
            root: self.builder.finish(),
        }
    }

    fn parse_block(&mut self) {
        let token = if let Some(tok) = self.tokenizer.peek() {
            tok
        } else {
            return;
        };

        match token.kind {
            TokenKind::Hash => {
                let mut level = 0;
                while let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Hash {
                        level += 1;
                        self.tokenizer.advance();
                    } else {
                        break;
                    }
                }

                let kind = match level {
                    1 => BlockKind::Header1,
                    2 => BlockKind::Header2,
                    3 => BlockKind::Header3,
                    4 => BlockKind::Header4,
                    5 => BlockKind::Header5,
                    _ => BlockKind::Header6,
                };

                self.builder.start_node(NodeKind::Block(kind).into());

                // skip whitespace if it exists
                // FIXME: parse as whitespace tokens
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Whitespace {
                        self.tokenizer.advance();
                    }
                }

                self.parse_inline(&|t| t.kind == TokenKind::Newline);

                // FIXME: parse as newline token
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Newline {
                        self.tokenizer.advance();
                    }
                }

                self.builder.finish_node();
            }

            TokenKind::Backticks(n) if n >= 3 => {
                // FIXME: parse as syntax text
                self.tokenizer.advance();
                self.builder
                    .start_node(NodeKind::Block(BlockKind::Codeblock).into());

                // peek text (language)
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Text {
                        let text = self.tokenizer.text(tok.span).to_string();
                        self.builder
                            .token(NodeKind::Text(TextKind::CodeblockLang).into(), &text);
                        self.tokenizer.advance();
                    }
                }

                // consume newline
                // FIXME: parse as newline token
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Newline {
                        self.tokenizer.advance();
                    }
                }

                // read until matching backticks or eof
                let mut content = String::new();
                while let Some(tok) = self.tokenizer.peek() {
                    if let TokenKind::Backticks(m) = tok.kind {
                        if m == n {
                            break;
                        }
                    }
                    self.tokenizer.advance();
                    content.push_str(self.tokenizer.text(tok.span));
                }

                if !content.is_empty() {
                    self.builder
                        .token(NodeKind::Text(TextKind::Text).into(), &content);
                }

                if let Some(tok) = self.tokenizer.peek() {
                    if let TokenKind::Backticks(m) = tok.kind {
                        if m == n {
                            // FIXME: parse backticks as syntax text
                            self.tokenizer.advance();
                        }
                    }
                }

                self.builder.finish_node();
            }
            TokenKind::AngleClose => {
                // FIXME: parse > as syntax text
                self.tokenizer.advance();
                self.builder
                    .start_node(NodeKind::Block(BlockKind::Blockquote).into());

                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Whitespace {
                        // FIXME: parse as whitespace tokens
                        self.tokenizer.advance();
                    }
                }

                self.parse_block();

                self.builder.finish_node();
            }
            _ => {
                self.builder
                    .start_node(NodeKind::Block(BlockKind::Paragraph).into());

                self.parse_inline(&|t| t.kind == TokenKind::Newline);

                // FIXME: parse as newline token
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Newline {
                        self.tokenizer.advance();
                    }
                }

                self.builder.finish_node();
            }
        }
    }
}
