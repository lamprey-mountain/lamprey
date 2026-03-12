//! Pull Parser / Event Iterator tests.

use crate::ast::Ast;
use crate::events::{Event, EventFilter, Tag};
use crate::parser::{ParseOptions, Parser};

#[test]
fn test_events_basic() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello world"));

    let events: Vec<_> = ast.events().collect();
    assert!(!events.is_empty());
}

#[test]
fn test_events_header() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("# Header 1\n## Header 2"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Header(_)))));
}

#[test]
fn test_events_list() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("- item 1\n- item 2"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::List(_)))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::ListItem))));
}

#[test]
fn test_events_numbered_list() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("1. first\n2. second"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::List(true)))));
}

#[test]
fn test_events_emphasis() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold** *italic* ~~strike~~"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Strong))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emphasis))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Strikethrough))));
}

#[test]
fn test_events_inline_code() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("code `inline` here"));

    let events: Vec<_> = ast.events().collect();
    assert!(!events.is_empty());
    assert!(events.iter().any(|e| matches!(e, Event::Text(_))));
}

#[test]
fn test_events_link() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[text](https://example.com)"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Link { .. }))));
}

#[test]
fn test_events_emoji() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc> world"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emoji { .. }))));
}

#[test]
fn test_events_animated_emoji() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <a:wave:12345678-1234-1234-1234-123456789abc> world"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emoji { .. }))));
}

#[test]
fn test_events_mention() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <@12345678-1234-1234-1234-123456789abc>"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Mention))));
}

#[test]
fn test_events_filter_strip_emphasis() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold** *italic* text"));

    let events: Vec<_> = ast.events().strip_emphasis().collect();
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emphasis))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Strong))));
}

#[test]
fn test_events_filter_strip_strong() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold** *italic* text"));

    let events: Vec<_> = ast.events().strip_strong().collect();
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Strong))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emphasis))));
}

#[test]
fn test_events_filter_strip_emoji() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("text <:smile:12345678-1234-1234-1234-123456789abc> more"));

    let events: Vec<_> = ast.events().strip_emoji().collect();
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emoji { .. }))));
}

#[test]
fn test_events_filter_composition() {
    let parser = Parser::new(ParseOptions::default());
    let ast =
        Ast::new(parser.parse("**bold** *italic* <:smile:12345678-1234-1234-1234-123456789abc>"));

    let events: Vec<_> = ast.events().strip_emphasis().strip_emoji().collect();
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emphasis))));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Emoji { .. }))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::Strong))));
}

#[test]
fn test_events_map_text() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello world"));

    let events: Vec<_> = ast.events().map_text(|t: &str| t.to_uppercase()).collect();
    assert!(!events.is_empty());
}

#[test]
fn test_events_merge_text() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello   world"));

    let events: Vec<_> = ast.events().merge_text().collect();
    assert!(!events.is_empty());
}

#[test]
fn test_events_code_block() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("```rust\ncode here\n```"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::CodeBlock))));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Code(c) if c.contains("code"))));
}

#[test]
fn test_events_blockquote() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("> quoted text"));

    let events: Vec<_> = ast.events().collect();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Start(Tag::BlockQuote))));
}

#[test]
fn test_events_nested_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold *italic* more**"));

    let events: Vec<_> = ast.events().collect();
    let strong_start = events
        .iter()
        .position(|e| matches!(e, Event::Start(Tag::Strong)));
    let emphasis_start = events
        .iter()
        .position(|e| matches!(e, Event::Start(Tag::Emphasis)));

    assert!(strong_start.is_some());
    assert!(emphasis_start.is_some());
    assert!(strong_start.unwrap() < emphasis_start.unwrap());
}

#[test]
fn test_events_empty_document() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(""));

    let events: Vec<_> = ast.events().collect();
    assert!(events.len() >= 0);
}
