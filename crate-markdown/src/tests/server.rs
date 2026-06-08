use crate::{parser::Parser, transform::StripEmoji};

#[test]
fn parse_mention_ids() {
    todo!()
}

#[test]
fn strip_emoji() {
    let source = "hello <:foo:00000000-0000-0000-0000-000000000001> world <:bar:00000000-0000-0000-0000-000000000002> test";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    let transformer = StripEmoji {
        allowed: vec![uuid::uuid!("00000000-0000-0000-0000-000000000002")],
    };

    let transformed = parsed.transform(&transformer);
    assert_eq!(
        transformed.to_markdown(),
        "hello :foo: world <:bar:00000000-0000-0000-0000-000000000002> test"
    );
    assert_eq!(transformed.to_plain(), "hello :foo: world :bar: test");
}

#[test]
fn parse_links() {
    todo!()
}
