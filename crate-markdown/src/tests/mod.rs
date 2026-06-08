use crate::parser::Parser;

mod basic;
mod tokenization;
mod util;

// TODO: write actual tests
#[test]
fn test() {
    let source = "hello *world* this [is](https://example.com) a **test**";
    let parser = Parser::new();
    let mut parsed = parser.parse(source);

    // TODO: test cursor
    // let mut cursor = parsed.cursor();
    // while let Some(node) = cursor.next() {
    //     if node.kind() == NodeKind::Inline(InlineKind::Emphasis) {
    //         let span = node.span();
    //         println!("found emphasis at bytes {}..{}", span.start, span.end);
    //     }
    // }

    assert_eq!(
        parsed.to_html(),
        "goodbye <em>world</em> this <a href=\"https://example.com\">is</a> a <strong>test</strong>"
    );

    parsed.edit((0, 5).into(), "goodbye");

    assert_eq!(
        parsed.to_plain(),
        "goodbye *world* this [is](https://example.com) a **test**"
    );
}
