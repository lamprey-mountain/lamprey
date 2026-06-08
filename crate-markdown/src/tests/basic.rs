use crate::parser::Parser;

#[test]
fn test_plain_text() {
    let parser = Parser::new();
    let parsed = parser.parse("hello world");
    assert_eq!(parsed.to_html(), "hello world");
    assert_eq!(parsed.to_markdown(), "hello world");
    assert_eq!(parsed.to_plain(), "hello world");
}

#[test]
fn test_emphasis() {
    let parser = Parser::new();
    let parsed = parser.parse("hello *world*");
    assert_eq!(parsed.to_html(), "hello <em>world</em>");
    assert_eq!(parsed.to_markdown(), "hello *world*");
    assert_eq!(parsed.to_plain(), "hello world");
}
