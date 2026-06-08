use crate::{
    ast::{
        block::{Block, Document},
        AstNode,
    },
    parser::Parser,
};

#[test]
fn test_plain_text() {
    let parser = Parser::new();
    let parsed = parser.parse("hello world");
    assert_eq!(parsed.to_html(), "<p>hello world</p>");
    assert_eq!(parsed.to_markdown(), "hello world");
    assert_eq!(parsed.to_plain(), "hello world");
}

#[test]
fn test_emphasis() {
    let parser = Parser::new();
    let parsed = parser.parse("hello *world*");
    assert_eq!(parsed.to_html(), "<p>hello <em>world</em></p>");
    assert_eq!(parsed.to_markdown(), "hello *world*");
    assert_eq!(parsed.to_plain(), "hello world");
}
