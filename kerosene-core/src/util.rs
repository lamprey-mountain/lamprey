use lamprey_markdown::{
    Parser,
    ast::{block::Block, inline::Inline},
};
use unicode_segmentation::UnicodeSegmentation;

/// calculate the effective length of a piece of text
// TODO: verify that this makes sense and doesnt allow suspicious
// TODO: use this for validation
// NOTE: i need to also enforce a max bytes length too?
// do i want to put this logic inside lamprey-markdown?
fn text_len(s: &str) -> usize {
    // TODO: normalize text to nfc

    let parser = Parser::new();
    let parsed = parser.parse(s);
    let document = parsed.document();

    for block in document.children() {
        match block {
            Block::Header(_header) => todo!(),
            Block::Paragraph(paragraph) => {
                for inline in paragraph.children() {
                    let _ = match inline {
                        // syntax should still count as chars (ie. asterisks surrounding strong)
                        Inline::Strong(_strong) => todo!(),
                        Inline::Emphasis(_emphasis) => todo!(),
                        Inline::Strikethrough(_strikethrough) => todo!(),
                        Inline::Spoiler(_spoiler) => todo!(),
                        Inline::Code(_code) => todo!(),
                        Inline::Link(_link) => todo!(),

                        // TODO: make cjk count as 2 units per grapheme
                        Inline::Text(text) => text.text().graphemes(true).count(),
                        Inline::Mention(_) => 24,
                        Inline::Timestamp(_) => 8,
                        Inline::CustomEmoji(_) => 2,
                        Inline::UnicodeEmoji(_) => 2,
                    };
                }
            }
            Block::Blockquote(_blockquote) => todo!(),
            Block::Codeblock(_codeblock) => todo!(),
            Block::List(_list) => todo!(),
            Block::Table(_table) => todo!(),
        }
    }

    todo!()
}
