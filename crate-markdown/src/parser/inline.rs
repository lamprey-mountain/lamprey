use crate::parser::ParseContext;
use crate::prelude::*;
use crate::tokenizer::Tokenizer;

impl<'a> ParseContext<'a> {
    pub(crate) fn parse_inline<F: FnMut(&mut Tokenizer<'a>) -> bool>(&mut self, stop: &mut F) {
        while let Some(_tok) = self.tokenizer.peek() {
            if stop(&mut self.tokenizer) {
                break;
            }

            let tok = self
                .tokenizer
                .advance()
                .expect("a token was successfully peeked");

            match tok.kind {
                // parse a codeblock
                TokenKind::Backtick => {
                    self.builder
                        .start_node(NodeKind::Inline(InlineKind::Code).into());
                    self.parse_inline(stop);
                    self.builder.finish_node();
                }

                // parse strong/emphasis
                TokenKind::Asterisk1 | TokenKind::Asterisk2 | TokenKind::Asterisk3 => {
                    // maybe split into multiple match arms
                    todo!()
                }

                // handle escape
                TokenKind::Backslash => {
                    todo!()
                }

                // Tilde2 strikethrough
                // Pipe2 spoiler
                // Url link (automatic)
                // BracketOpen link
                // AngleOpen link

                // TODO: finish implementing other inline tokens

                // otherwise try to parse the token as text
                _ => {
                    let text = self.tokenizer.text(tok.span).to_string();
                    // TODO: use correct TextKind instead of always TextKind::Text
                    self.builder
                        .token(NodeKind::Text(TextKind::Text).into(), &text);
                }
            }
        }
    }
}
