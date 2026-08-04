use crate::ast::list::ListKind;
use crate::lexer::Token;
use crate::parser::ParseContext;
use crate::parser::helpers::{Draft, DraftError};
use crate::prelude::*;

// TODO: create a macro or some sort of system to make parsing more ergonomic

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
                let mut hashes = String::new();
                while let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Hash {
                        level += 1;
                        hashes.push_str(self.tokenizer.text(tok.span));
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
                self.builder
                    .token(NodeKind::Text(TextKind::HeaderHashes).into(), &hashes);

                // skip whitespace if it exists
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Whitespace {
                        let text = self.tokenizer.text(tok.span).to_string();
                        self.builder
                            .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                        self.tokenizer.advance();
                    }
                }

                self.parse_inline(&|t| t.kind == TokenKind::Newline);

                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Newline {
                        self.builder
                            .token(NodeKind::Text(TextKind::Padding).into(), "\n");
                        self.tokenizer.advance();
                    }
                }

                self.builder.finish_node();
            }

            TokenKind::Backticks(n) if n >= 3 => {
                let backticks = self.tokenizer.advance().unwrap();
                let backticks_text = self.tokenizer.text(backticks.span).to_string();
                self.builder
                    .start_node(NodeKind::Block(BlockKind::Codeblock).into());
                self.builder
                    .token(NodeKind::Text(TextKind::Syntax).into(), &backticks_text);

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
                if let Some(tok) = self.tokenizer.peek() {
                    if tok.kind == TokenKind::Newline {
                        self.builder
                            .token(NodeKind::Text(TextKind::Padding).into(), "\n");
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

                let trimmed = content.strip_suffix("\n").unwrap_or(&content);
                if !trimmed.is_empty() {
                    self.builder
                        .token(NodeKind::Text(TextKind::Text).into(), trimmed);
                    if trimmed.len() != content.len() {
                        self.builder
                            .token(NodeKind::Text(TextKind::Padding).into(), "\n");
                    }
                }

                if let Some(tok) = self.tokenizer.peek() {
                    if let TokenKind::Backticks(m) = tok.kind {
                        if m == n {
                            let text = self.tokenizer.text(tok.span).to_string();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                            self.tokenizer.advance();
                        }
                    }
                }

                self.builder.finish_node();
            }

            TokenKind::AngleClose => {
                let tok = self.tokenizer.advance().unwrap();
                let text = self.tokenizer.text(tok.span).to_string();
                self.builder
                    .start_node(NodeKind::Block(BlockKind::Blockquote).into());
                self.builder
                    .token(NodeKind::Text(TextKind::Syntax).into(), &text);

                loop {
                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Whitespace {
                            let text = self.tokenizer.text(tok.span).to_string();
                            self.builder
                                .token(NodeKind::Text(TextKind::Padding).into(), &text);
                            self.tokenizer.advance();
                        }
                    }

                    self.parse_block();

                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::AngleClose {
                            let text = self.tokenizer.text(tok.span).to_string();
                            self.builder
                                .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                            self.tokenizer.advance();
                            continue;
                        }
                    }

                    break;
                }

                self.builder.finish_node();
            }
            _ => {
                if self.is_table() {
                    self.parse_table();
                } else {
                    if self.parse_list() {
                        return;
                    }

                    self.builder
                        .start_node(NodeKind::Block(BlockKind::Paragraph).into());

                    self.parse_inline(&|t| t.kind == TokenKind::Newline);

                    if let Some(tok) = self.tokenizer.peek() {
                        if tok.kind == TokenKind::Newline {
                            self.builder
                                .token(NodeKind::Text(TextKind::Newline).into(), "\n");
                            self.tokenizer.advance();
                        }
                    }

                    self.builder.finish_node();
                }
            }
        }
    }

    fn is_table(&self) -> bool {
        let mut lexer = self.tokenizer.clone();

        // skip whitespace
        if let Some(tok) = lexer.peek() {
            if tok.kind == TokenKind::Whitespace {
                lexer.advance();
            }
        }

        if let Some(tok) = lexer.peek() {
            if tok.kind != TokenKind::Pipe && tok.kind != TokenKind::Pipe2 {
                return false;
            }
        } else {
            return false;
        }

        loop {
            let mut has_dash = false;
            let mut has_pipe = false;
            let mut valid_align_chars = true;
            let mut is_empty = true;

            while let Some(tok) = lexer.advance() {
                if tok.kind == TokenKind::Newline {
                    break;
                }
                is_empty = false;
                match tok.kind {
                    TokenKind::Dash => has_dash = true,
                    TokenKind::Pipe | TokenKind::Pipe2 => has_pipe = true,
                    TokenKind::Colon | TokenKind::Whitespace => {}
                    _ => valid_align_chars = false,
                }
            }

            if is_empty {
                return false;
            }

            if valid_align_chars && has_dash && has_pipe {
                return true;
            }
        }
    }

    fn is_alignment_row(&self) -> bool {
        let mut lexer = self.tokenizer.clone();
        let mut has_dash = false;
        let mut valid_chars = true;
        let mut align_row_empty = true;

        while let Some(tok) = lexer.advance() {
            if tok.kind == TokenKind::Newline {
                break;
            }
            align_row_empty = false;
            match tok.kind {
                TokenKind::Dash => has_dash = true,
                TokenKind::Pipe | TokenKind::Pipe2 => {}
                TokenKind::Colon | TokenKind::Whitespace => {}
                _ => valid_chars = false,
            }
        }

        !align_row_empty && valid_chars && has_dash
    }

    fn parse_table(&mut self) {
        self.builder
            .start_node(NodeKind::Block(BlockKind::Table).into());

        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Newline {
                break;
            }

            if self.is_alignment_row() {
                self.parse_alignment_row();
            } else {
                self.parse_table_row();
            }

            if let Some(tok) = self.tokenizer.peek() {
                if tok.kind == TokenKind::Newline {
                    self.builder
                        .token(NodeKind::Text(TextKind::Newline).into(), "\n");
                    self.tokenizer.advance();
                } else {
                    break;
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_table_row(&mut self) {
        self.builder
            .start_node(NodeKind::Block(BlockKind::TableRow).into());

        if let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Pipe || tok.kind == TokenKind::Pipe2 {
                let text = self.tokenizer.text(tok.span).to_string();
                self.builder
                    .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                self.tokenizer.advance();
            }
        }

        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Newline {
                break;
            }

            self.builder
                .start_node(NodeKind::Block(BlockKind::TableCell).into());

            if let Some(tok) = self.tokenizer.peek() {
                if tok.kind == TokenKind::Whitespace {
                    let text = self.tokenizer.text(tok.span).to_string();
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                    self.tokenizer.advance();
                }
            }

            self.parse_inline(&|t| {
                t.kind == TokenKind::Pipe
                    || t.kind == TokenKind::Pipe2
                    || t.kind == TokenKind::Newline
            });

            self.builder.finish_node();

            if let Some(t) = self.tokenizer.peek() {
                if t.kind == TokenKind::Pipe || t.kind == TokenKind::Pipe2 {
                    let text = self.tokenizer.text(t.span).to_string();
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                    self.tokenizer.advance();
                } else if t.kind == TokenKind::Newline {
                    break;
                }
            }
        }

        self.builder.finish_node();
    }

    fn parse_alignment_row(&mut self) {
        self.builder
            .start_node(NodeKind::Block(BlockKind::TableRow).into());

        if let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Pipe || tok.kind == TokenKind::Pipe2 {
                let text = self.tokenizer.text(tok.span).to_string();
                self.builder
                    .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                self.tokenizer.advance();
            }
        }

        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Newline {
                break;
            }

            self.builder
                .start_node(NodeKind::Block(BlockKind::TableCell).into());

            let mut align_text = String::new();
            while let Some(t) = self.tokenizer.peek() {
                if t.kind == TokenKind::Pipe
                    || t.kind == TokenKind::Pipe2
                    || t.kind == TokenKind::Newline
                {
                    break;
                }
                align_text.push_str(self.tokenizer.text(t.span));
                self.tokenizer.advance();
            }

            if !align_text.is_empty() {
                self.builder
                    .token(NodeKind::Text(TextKind::TableAlignment).into(), &align_text);
            }

            self.builder.finish_node();

            if let Some(t) = self.tokenizer.peek() {
                if t.kind == TokenKind::Pipe || t.kind == TokenKind::Pipe2 {
                    let text = self.tokenizer.text(t.span).to_string();
                    self.builder
                        .token(NodeKind::Text(TextKind::Syntax).into(), &text);
                    self.tokenizer.advance();
                } else if t.kind == TokenKind::Newline {
                    break;
                }
            }
        }

        self.builder.finish_node();
    }

    /// attempt to parse a markdown list
    ///
    /// returns true if a list was successfully parsed
    fn parse_list(&mut self) -> bool {
        // find out what kind of list this is
        let mut probe = self.tokenizer.clone();
        probe.consume_whitespace();
        let kind = match probe.peek_kind() {
            Some(TokenKind::Number) => ListKind::Ordered,
            Some(TokenKind::Asterisk1) => ListKind::Unordered,
            Some(TokenKind::Dash) => {
                probe.advance();

                if probe.consume_whitespace() == 0 {
                    return false;
                }

                if probe.peek_kind() == Some(TokenKind::BracketOpen) {
                    ListKind::Tasks
                } else {
                    ListKind::Unordered
                }
            }

            _ => return false,
        };

        let mut started = false;

        loop {
            let Ok(draft) = self.parse_list_prefix(kind) else {
                break;
            };

            if !started {
                self.builder.start_node(kind.node_kind().into());
                started = true;
            }

            self.builder
                .start_node(NodeKind::Block(BlockKind::ListItem).into());

            let (tokens, lexer) = draft.into_tokens_lexer();
            for (kind, span) in tokens {
                self.builder.token(kind.into(), self.tokenizer.text(span));
            }
            self.tokenizer = lexer;

            // TODO: parse list items as blocks?
            self.parse_inline(&|t| t.kind == TokenKind::Newline);

            // skip over trailing newline
            if let Some(tok) = self.tokenizer.peek() {
                if tok.kind == TokenKind::Newline {
                    self.builder
                        .token(NodeKind::Text(TextKind::Newline).into(), "\n");
                    self.tokenizer.advance();
                }
            }

            self.builder.finish_node();
        }

        if !started {
            return false;
        }

        self.builder.finish_node();
        true
    }

    /// attempt to parse a markdown list prefix
    ///
    /// returns true if a list prefix was successfully parsed
    fn parse_list_prefix(&mut self, kind: ListKind) -> Result<Draft<'a>, DraftError> {
        let mut draft = Draft::new(self.tokenizer.clone());

        // skip over leading whitespace
        // TODO: maybe add TextKind variant specifically for list item "pre-padding"
        let _ = draft.consume_whitespace(TextKind::Padding);

        match kind {
            ListKind::Ordered => {
                let num = draft.read(TokenKind::Number)?;
                let dot = draft.read(TokenKind::Dot)?;
                draft.push(TextKind::ListPrefix, (num.start, dot.end).into());
            }
            ListKind::Unordered => {
                let tok = draft
                    .read(TokenKind::Asterisk1)
                    .or_else(|_| draft.read(TokenKind::Dash))?;

                draft.push(TextKind::ListPrefix, tok);
            }

            ListKind::Tasks => {
                draft.consume(TokenKind::Dash, TextKind::ListPrefix)?;
                _ = draft.consume_whitespace(TextKind::Padding);
                draft.consume(TokenKind::BracketOpen, TextKind::ListPrefix)?;

                // FIXME: parse this whitespace as TaskMark instead of Padding if task mark is empty
                _ = draft.consume_whitespace(TextKind::Padding);

                match draft.advance() {
                    Some(Token {
                        kind: TokenKind::BracketClose,
                        span,
                    }) => {
                        // TODO: maybe push mark (see fixme above)
                        // draft.push(TextKind::TaskMark, span);

                        // empty or only whitespace
                        draft.push(TextKind::ListPrefix, span);
                    }
                    Some(Token { span, .. }) => {
                        // TODO: parse and merge multiple tokens instead of only the next one?
                        draft.push(TextKind::TaskMark, span);
                        _ = draft.consume_whitespace(TextKind::Padding);
                        draft.consume(TokenKind::BracketClose, TextKind::ListPrefix)?;
                    }
                    None => {
                        // unexpected eof
                        return Err(DraftError::Mismatch);
                    }
                }
            }
        };

        // skip over padding whitespace
        draft.consume_whitespace(TextKind::Padding)?;

        Ok(draft)
    }
}
