use crate::parser::ParseContext;
use crate::prelude::*;

impl<'a> ParseContext<'a> {
    pub fn parse_document(mut self) -> Tree {
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
                // let level = count number of TokenKind::Hash tokens

                // let kind = match level {
                //     1 => BlockKind::Header1,
                //     2 => BlockKind::Header2,
                //     3 => BlockKind::Header3,
                //     4 => BlockKind::Header4,
                //     5 => BlockKind::Header5,
                //     _ => BlockKind::Header6,
                // };

                // self.builder.start_node(NodeKind::Block(kind).into());

                // skip whitespace if it exists
                // parse inline until newline reached

                // self.builder.finish_node();
                todo!()
            }

            // FIXME: handle more than 3 backticks
            TokenKind::Backtick3 => {
                self.builder
                    .start_node(NodeKind::Block(BlockKind::Codeblock).into());

                // peek text (language) or newline
                // TextKind::CodeblockLang

                // read until matching backticks or eof
                // while let Some(tok) = self.tokenizer.next() {}
                // parse as text; special case Backtick3 and Escape (for escaping backticks)
                // self.builder
                //     .token(NodeKind::Text(TextKind::Text).into(), &text);

                self.builder.finish_node();

                todo!()
            }
            TokenKind::AngleClose => {
                // self.builder
                //     .start_node(NodeKind::Block(BlockKind::Blockquote).into());
                // skip whitespace
                // parse paragraph
                // self.builder.finish_node();
                todo!()
            }
            _ => {
                // self.builder
                //     .start_node(NodeKind::Block(BlockKind::Paragraph).into());

                // parse inline until a newline is reached

                // self.builder.finish_node();
                todo!()
            }
        }
    }
}
