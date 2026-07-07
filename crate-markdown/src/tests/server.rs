use crate::{parser::Parser, query::QueryableExt, transform::StripEmoji};

#[test]
fn parse_mentions() {
    let parser = Parser::new();
    let source = "<@00000000-0000-0000-0000-000000000000> <&00000000-0000-0000-0000-000000000001> <#00000000-0000-0000-0000-000000000002> <@everyone>";
    let parsed = parser.parse(source);
    let mentions: Vec<_> = parsed.tree().iter_mentions().collect();
    assert_eq!(mentions.len(), 4);
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
    let parser = Parser::new();
    let source = "<https://example.com> <not-a-url>";
    let parsed = parser.parse(source);
    let links: Vec<_> = parsed.tree().iter_links().collect();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].href(), "https://example.com");
}
