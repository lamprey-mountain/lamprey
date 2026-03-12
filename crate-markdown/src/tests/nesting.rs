//! Complex nesting tests.

use crate::parser::{ParseOptions, Parser, SyntaxKind};

#[test]
fn test_bold_inside_italic() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("*italic **bold** more*");
    let root = parsed.syntax();

    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);

    assert!(has_emphasis, "Should have italic");
    assert!(has_strong, "Should have bold inside italic");
}

#[test]
fn test_italic_inside_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold *italic* more**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

    assert!(has_strong, "Should have bold");
    assert!(has_emphasis, "Should have italic inside bold");
}

#[test]
fn test_strikethrough_inside_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold ~~deleted~~ more**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_strike = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Strikethrough);

    assert!(has_strong, "Should have bold");
    assert!(has_strike, "Should have strikethrough inside bold");
}

#[test]
fn test_link_inside_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold [link](url) more**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);

    assert!(has_strong, "Should have bold");
    assert!(has_link, "Should have link inside bold");
}

#[test]
fn test_code_inside_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold `code` more**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_code = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::InlineCode);

    assert!(has_strong, "Should have bold");
    assert!(has_code, "Should have code inside bold");
}

#[test]
fn test_all_inline_formats_nested() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold *italic ~~strike~~ more* end**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    let has_strike = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Strikethrough);

    assert!(has_strong, "Should have bold");
    assert!(has_emphasis, "Should have italic");
    assert!(has_strike, "Should have strikethrough");
}

#[test]
fn test_mention_inside_bold() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**hello @12345678-1234-1234-1234-123456789abc world**");
    let root = parsed.syntax();

    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
    let has_mention = root.descendants().any(|n| n.kind() == SyntaxKind::Mention);

    assert!(has_strong, "Should have bold");
    assert!(has_mention, "Should have mention inside bold");
}

#[test]
fn test_emoji_inside_italic() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("*hello <:smile:12345678-1234-1234-1234-123456789abc> world*");
    let root = parsed.syntax();

    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    let has_emoji = root.descendants().any(|n| n.kind() == SyntaxKind::Emoji);

    assert!(has_emphasis, "Should have italic");
    assert!(has_emoji, "Should have emoji inside italic");
}
