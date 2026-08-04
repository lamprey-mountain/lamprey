use crate::{
    lexer::{Lexer, Token},
    parser::ParseContext,
    prelude::*,
};

impl<'a> ParseContext<'a> {
    /// consume over as much whitespace as possible
    ///
    /// returns whether any whitespace was consumed
    pub fn consume_whitespace(&mut self, kind: TextKind) -> bool {
        let mut text = String::new();
        while let Some(tok) = self.tokenizer.peek() {
            if tok.kind == TokenKind::Whitespace {
                text.push_str(self.tokenizer.text(tok.span));
                self.tokenizer.advance();
            } else {
                break;
            }
        }

        if text.is_empty() {
            false
        } else {
            self.builder.token(NodeKind::Text(kind).into(), &text);
            true
        }
    }

    /// consume a token of a specific kind
    ///
    /// returns whether a token was consumed
    pub fn consume(&mut self, token_kind: TokenKind, node_kind: TextKind) -> bool {
        if let Some(tok) = self.tokenizer.peek() {
            if tok.kind == token_kind {
                let text = self.tokenizer.text(tok.span);
                self.builder.token(NodeKind::Text(node_kind).into(), text);
                self.tokenizer.advance();
            }
        }

        false
    }
}

pub struct Draft<'source> {
    lexer: Lexer<'source>,
    tokens: Vec<(NodeKind, Span)>,
}

pub enum DraftError {
    Mismatch,
}

impl<'source> Draft<'source> {
    pub fn new(lexer: Lexer<'source>) -> Self {
        Self {
            lexer,
            tokens: vec![],
        }
    }

    /// manually push a token
    pub fn push(&mut self, kind: TextKind, span: Span) {
        self.tokens.push((NodeKind::Text(kind), span));
    }

    /// consume over as much whitespace as possible
    pub fn consume_whitespace(&mut self, kind: TextKind) -> Result<(), DraftError> {
        let Some(first) = self.lexer.peek() else {
            return Err(DraftError::Mismatch);
        };

        let mut text = String::new();
        let mut span = first.span;

        while let Some(tok) = self.lexer.peek() {
            if tok.kind == TokenKind::Whitespace {
                text.push_str(self.lexer.text(tok.span));
                span.end = tok.span.end;
                self.lexer.advance();
            } else {
                break;
            }
        }

        self.tokens.push((NodeKind::Text(kind), span));

        Ok(())
    }

    /// consume a token of a specific kind
    pub fn consume(
        &mut self,
        token_kind: TokenKind,
        node_kind: TextKind,
    ) -> Result<(), DraftError> {
        if let Some(tok) = self.lexer.peek() {
            if tok.kind == token_kind {
                self.tokens.push((NodeKind::Text(node_kind), tok.span));
                self.lexer.advance();
                return Ok(());
            }
        }

        Err(DraftError::Mismatch)
    }

    /// read a token of a specific kind
    ///
    /// similar to consume, but returns a span instead of immediately creating a node
    pub fn read(&mut self, token_kind: TokenKind) -> Result<Span, DraftError> {
        if let Some(tok) = self.lexer.peek() {
            if tok.kind == token_kind {
                self.lexer.advance();
                return Ok(tok.span);
            }
        }

        Err(DraftError::Mismatch)
    }

    pub fn advance(&mut self) -> Option<Token> {
        self.lexer.advance()
    }

    pub fn into_tokens_lexer(self) -> (Vec<(NodeKind, Span)>, Lexer<'source>) {
        (self.tokens, self.lexer)
    }
}
