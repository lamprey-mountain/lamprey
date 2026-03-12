//! Escape sequence tests.

use crate::ast::Ast;
use crate::parser::{ParseOptions, Parser, SyntaxKind};
use crate::render::PlainTextReader;

#[test]
fn test_escape_asterisk() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\*not italic\\*");
    let root = parsed.syntax();

    let escape_count = root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Escape)
        .count();

    assert_eq!(escape_count, 2, "Should have two escape sequences");

    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    assert!(
        !has_emphasis,
        "Escaped asterisks should not create emphasis"
    );
}

#[test]
fn test_escape_backslash() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("path\\\\to\\\\file");
    let root = parsed.syntax();

    let escape_count = root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Escape)
        .count();

    assert!(escape_count >= 1, "Should have escape sequences");
}

#[test]
fn test_escape_bracket() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\[not a link\\]");
    let root = parsed.syntax();

    let escape_count = root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Escape)
        .count();

    assert_eq!(escape_count, 2, "Should have two escape sequences");

    let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);
    assert!(!has_link, "Escaped brackets should not create a link");
}

#[test]
fn test_plain_text_with_escapes() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("hello world");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains("hello"), "Should contain hello");
    assert!(result.contains("world"), "Should contain world");
}

#[test]
fn test_escape_in_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold with \\* asterisk**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    assert!(has_strong, "Should have bold");

    let has_escape = root.descendants().any(|n| n.kind() == SyntaxKind::Escape);
    assert!(has_escape, "Should have escape inside bold");
}

#[test]
fn test_escape_multiple_in_row() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\*\\*\\*");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert_eq!(result, "***", "Should have three asterisks");
}

#[test]
fn test_escape_mixed_chars() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\*\\[\\]\\#");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains("*"), "Should contain asterisk");
    assert!(result.contains("["), "Should contain bracket");
    assert!(result.contains("]"), "Should contain close bracket");
    assert!(result.contains("#"), "Should contain hash");
    assert!(!result.contains('\\'), "Should not contain backslash");
}

#[test]
fn test_escape_backtick_plain_text() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\`not code\\`");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains('`'), "Should contain backtick");
    assert!(result.contains("code"), "Should contain code");
    assert!(result.contains("not"), "Should contain not");
}

#[test]
fn test_escape_in_link() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\[not link\\](url)");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains('['), "Should contain bracket");
    assert!(result.contains(']'), "Should contain close bracket");
}

#[test]
fn test_escape_at_end_of_text() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("text\\");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains("text"), "Should contain text");
}

#[test]
fn test_escape_newline() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("line1\\nline2");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains('n'), "Should contain n");
}

#[test]
fn test_escape_preserves_meaning() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\* \\[ \\] \\# \\`");
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);

    assert!(result.contains('*'), "Should contain asterisk");
    assert!(result.contains('['), "Should contain bracket");
    assert!(result.contains(']'), "Should contain close bracket");
    assert!(result.contains('#'), "Should contain hash");
    assert!(result.contains('`'), "Should contain backtick");
}

#[test]
fn test_escape_hash() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\# not a header");
    let root = parsed.syntax();

    let has_escape = root.descendants().any(|n| n.kind() == SyntaxKind::Escape);
    assert!(has_escape, "Should have escape");

    let has_header = root.descendants().any(|n| n.kind() == SyntaxKind::Header);
    assert!(!has_header, "Escaped hash should not create header");
}

#[test]
fn test_escape_dash() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\- not a list");
    let root = parsed.syntax();

    let has_escape = root.descendants().any(|n| n.kind() == SyntaxKind::Escape);
    assert!(has_escape, "Should have escape");

    let has_list = root.descendants().any(|n| n.kind() == SyntaxKind::List);
    assert!(!has_list, "Escaped dash should not create list");
}

#[test]
fn test_escape_backtick() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\`not code\\`");
    let root = parsed.syntax();

    let escape_count = root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Escape)
        .count();

    assert_eq!(escape_count, 2, "Should have two escapes");

    let has_inline_code = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::InlineCode);
    assert!(
        !has_inline_code,
        "Escaped backticks should not create inline code"
    );
}

#[test]
fn test_multiple_escapes_in_row() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("\\*\\*\\*");
    let root = parsed.syntax();

    let escape_count = root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Escape)
        .count();

    assert!(escape_count >= 2, "Should have multiple escapes");
}

#[test]
fn test_escape_at_end() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("text\\");
    let root = parsed.syntax();

    let has_document = root.descendants().any(|n| n.kind() == SyntaxKind::Document);
    assert!(has_document, "Should have document");
}
