//! Malformed input tests.

use crate::parser::{ParseOptions, Parser, SyntaxKind};

#[test]
fn test_unclosed_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**unclosed bold");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    assert!(!has_strong, "Unclosed bold should not create Strong node");

    let has_document = root.descendants().any(|n| n.kind() == SyntaxKind::Document);
    assert!(has_document, "Should have document");
}

#[test]
fn test_unclosed_italic() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("*unclosed italic");
    let root = parsed.syntax();

    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    assert!(
        !has_emphasis,
        "Unclosed italic should not create Emphasis node"
    );
}

#[test]
fn test_unclosed_strikethrough() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("~~unclosed strikethrough");
    let root = parsed.syntax();

    let has_strike = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Strikethrough);
    assert!(
        !has_strike,
        "Unclosed strikethrough should not create Strikethrough node"
    );
}

#[test]
fn test_unclosed_code() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("`unclosed code");
    let root = parsed.syntax();

    let has_code = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::InlineCode);
    assert!(!has_code, "Unclosed code should not create InlineCode node");
}

#[test]
fn test_unclosed_link() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("[unclosed link");
    let root = parsed.syntax();

    let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);
    assert!(!has_link, "Unclosed link should not create Link node");
}

#[test]
fn test_link_without_url() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("[text] no url");
    let root = parsed.syntax();

    let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);
    assert!(!has_link, "Link without URL should not create Link node");
}

#[test]
fn test_mismatched_delimiters() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold * mismatched");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

    assert!(
        !has_strong,
        "Mismatched delimiters should not create Strong"
    );
    assert!(
        !has_emphasis,
        "Mismatched delimiters should not create Emphasis"
    );
}

#[test]
fn test_single_asterisk() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("*");
    let root = parsed.syntax();

    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    assert!(!has_emphasis, "Single asterisk should not create Emphasis");
}

#[test]
fn test_triple_asterisk() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("***");
    let root = parsed.syntax();

    let has_document = root.descendants().any(|n| n.kind() == SyntaxKind::Document);
    assert!(has_document, "Should have document");
}

#[test]
fn test_empty_delimiters() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("****");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    assert!(has_strong, "Empty delimiters should still create node");
}

#[test]
fn test_only_special_chars() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**~~");
    let root = parsed.syntax();

    let has_root = root.kind() == SyntaxKind::Root;
    let has_document = root.children().any(|n| n.kind() == SyntaxKind::Document);

    assert!(has_root, "Should have Root");
    assert!(has_document, "Should have Document");
}

#[test]
fn test_very_nested_unclosed() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold *italic ~~strike");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    let has_strike = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Strikethrough);

    assert!(!has_strong, "Unclosed should not create Strong");
    assert!(!has_emphasis, "Unclosed should not create Emphasis");
    assert!(!has_strike, "Unclosed should not create Strikethrough");
}

#[test]
fn test_empty_string() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("");
    let root = parsed.syntax();

    let has_root = root.kind() == SyntaxKind::Root;
    let has_document = root.children().any(|n| n.kind() == SyntaxKind::Document);

    assert!(has_root, "Should have Root");
    assert!(has_document, "Should have Document");
}

#[test]
fn test_only_whitespace() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("   ");
    let root = parsed.syntax();

    let has_root = root.kind() == SyntaxKind::Root;
    assert!(has_root, "Should have Root even for whitespace only");
}

#[test]
fn test_only_newlines() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\n\n\n");
    let root = parsed.syntax();

    let has_root = root.kind() == SyntaxKind::Root;
    assert!(has_root, "Should have Root even for newlines only");
}
