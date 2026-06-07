use crate::{parser::Parser, tree::ElementKind};

mod util;

// TODO: write actual tests
// #[test]
// fn test() {
//     let source = "hello *world* this [is](https://example.com) a **test**";
//     let parser = Parser::new();
//     let mut parsed = parser.parse(source);

//     let mut cursor = parsed.cursor();
//     while let Some(node) = cursor.next() {
//         if node.kind() == ElementKind::MarkEmphasis {
//             let span = node.span();
//             println!("found emphasis at bytes {}..{}", span.start, span.end);
//         }
//     }

//     assert_eq!(
//         parsed.to_html(),
//         "goodbye <em>world</em> this <a href=\"https://example.com\">is</a> a <strong>test</strong>"
//     );

//     parsed.edit(0, 5, "goodbye");

//     assert_eq!(
//         parsed.to_plain(),
//         "goodbye *world* this [is](https://example.com) a **test**"
//     );
// }
