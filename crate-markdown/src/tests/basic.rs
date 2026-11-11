use crate::parser::Parser;

#[test]
fn test_plain_text() {
    let parser = Parser::new();
    let parsed = parser.parse("hello world");
    assert_eq!(parsed.to_html(), "<p>hello world</p>");
    assert_eq!(parsed.to_markdown(), "hello world");
    assert_eq!(parsed.to_plain(), "hello world");
}

#[test]
fn test_timestamp_style() {
    let parser = Parser::new();

    let html = |s: &str| parser.parse(s).to_html();

    assert_eq!(
        html("<t:1618953630:d>"),
        "<p><time datetime=\"1618953630\" data-style=\"d\"></time></p>"
    );
    assert_eq!(
        html("<t:1618953630:D>"),
        "<p><time datetime=\"1618953630\" data-style=\"D\"></time></p>"
    );
    assert_eq!(
        html("<t:1618953630:t>"),
        "<p><time datetime=\"1618953630\" data-style=\"t\"></time></p>"
    );
    assert_eq!(
        html("<t:1618953630:dt>"),
        "<p><time datetime=\"1618953630\" data-style=\"dt\"></time></p>"
    );
    assert_eq!(
        html("<t:1618953630>"),
        "<p><time datetime=\"1618953630\" data-style=\"r\"></time></p>"
    );
}

#[test]
fn test_emphasis() {
    let parser = Parser::new();
    let parsed = parser.parse("hello *world*");
    assert_eq!(parsed.to_html(), "<p>hello <em>world</em></p>");
    assert_eq!(parsed.to_markdown(), "hello *world*");
    assert_eq!(parsed.to_plain(), "hello world");
}
