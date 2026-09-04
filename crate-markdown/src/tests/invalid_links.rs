use crate::parser::Parser;
use crate::prelude::*;
use crate::query::{Decoration, DecorationKind, QueryableExt};

#[test]
fn test_incomplete_link_only_brackets() {
    let source = "[link]";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<p>[link]</p>");
    assert_eq!(parsed.to_markdown(), "[link]");
    assert_eq!(parsed.to_plain(), "[link]");

    let decos: Vec<_> = parsed.tree().iter_decorations(None).collect();
    assert_eq!(
        decos,
        [
            Decoration {
                span: Span { start: 0, end: 1 },
                kind: DecorationKind::Syntax,
            },
            Decoration {
                span: Span { start: 4, end: 5 },
                kind: DecorationKind::Syntax,
            }
        ]
    )
}

#[test]
fn test_incomplete_link_with_opening_paren() {
    let source = "[link](";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<p>[link](</p>");
    assert_eq!(parsed.to_markdown(), "[link](");
    assert_eq!(parsed.to_plain(), "[link](");
    // TODO: assert that this is still decorated like a valid link
}

#[test]
fn test_link_empty() {
    let source = "[link]()";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<p>[link]()</p>");
    assert_eq!(parsed.to_markdown(), "[link]()");
    assert_eq!(parsed.to_plain(), "[link]()");
}

#[test]
fn test_link_not_url() {
    let source = "[link](not a url)";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<p>[link](not a url)</p>");
    assert_eq!(parsed.to_markdown(), "[link](not a url)");
    assert_eq!(parsed.to_plain(), "[link](not a url)");
}

#[test]
fn test_link_nested() {
    let source = "[link https://example.com](https://bad.com)";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(
        parsed.to_html(),
        "<p><a href=\"https://bad.com\">link https://example.com</a></p>"
    );
    assert_eq!(
        parsed.to_markdown(),
        "[link https://example.com](https://bad.com)"
    );
    assert_eq!(
        parsed.to_plain(),
        "[link https://example.com](https://bad.com)"
    );
}

#[test]
fn test_link_nested_many() {
    let source = "[a [c](d)](b [e](f))";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<p>[a [c](d)](b [e](f))</p>");
    assert_eq!(parsed.to_markdown(), "[a [c](d)](b [e](f))");
    assert_eq!(parsed.to_plain(), "[a c(d)](b e(f))");
}

#[test]
fn test_link_nested_without_url() {
    let source = "[link https://example.com](not a url)";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(
        parsed.to_html(),
        "<p>[link https://example.com](not a url)</p>"
    );
    assert_eq!(
        parsed.to_markdown(),
        "[link https://example.com](not a url)"
    );
    assert_eq!(parsed.to_plain(), "[link https://example.com](not a url)");
}
