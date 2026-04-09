//! Parsing structure tests.

use crate::parser::{ParseOptions, Parser, SyntaxKind};
use rowan::{NodeOrToken, SyntaxNode};

#[test]
fn test_parse_bold_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**hello**");
    let root = parsed.syntax();

    let strong_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Strong)
        .expect("Should have Strong node");

    let mut has_open = false;
    let mut has_close = false;

    for child in strong_node.children_with_tokens() {
        if let NodeOrToken::Token(t) = child {
            if t.kind() == SyntaxKind::StrongDelimiter {
                if !has_open {
                    has_open = true;
                } else {
                    has_close = true;
                }
            }
        }
    }

    assert!(has_open, "Should have opening delimiter");
    assert!(has_close, "Should have closing delimiter");
}

#[test]
fn test_parse_italic_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("*hello*");
    let root = parsed.syntax();

    let emphasis_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Emphasis)
        .expect("Should have Emphasis node");

    let delimiter_count = emphasis_node
        .children_with_tokens()
        .filter(|child| {
            if let NodeOrToken::Token(t) = child {
                t.kind() == SyntaxKind::EmphasisDelimiter
            } else {
                false
            }
        })
        .count();

    assert_eq!(delimiter_count, 2, "Should have two emphasis delimiters");
}

#[test]
fn test_parse_strikethrough_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("~~hello~~");
    let root = parsed.syntax();

    let strike_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Strikethrough)
        .expect("Should have Strikethrough node");

    let delimiter_count = strike_node
        .children_with_tokens()
        .filter(|child| {
            if let NodeOrToken::Token(t) = child {
                t.kind() == SyntaxKind::StrikethroughDelimiter
            } else {
                false
            }
        })
        .count();

    assert_eq!(
        delimiter_count, 2,
        "Should have two strikethrough delimiters"
    );
}

#[test]
fn test_parse_code_block_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("```\ncode\n```");
    let root = parsed.syntax();

    let code_block = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::CodeBlock)
        .expect("Should have CodeBlock node");

    let fence_count = code_block
        .children()
        .filter(|child| child.kind() == SyntaxKind::CodeBlockFence)
        .count();
    let has_content = code_block
        .children()
        .any(|child| child.kind() == SyntaxKind::CodeBlockContent);

    assert!(fence_count >= 1, "Should have at least 1 CodeBlockFence");
    assert!(has_content, "Should have CodeBlockContent");
}

#[test]
fn test_parse_blockquote_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("> quoted text");
    let root = parsed.syntax();

    let has_blockquote = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::BlockQuote);

    assert!(has_blockquote, "Should have BlockQuote node");
}

#[test]
fn test_parse_link_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("[text](https://example.com)");
    let root = parsed.syntax();

    let link_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Link)
        .expect("Should have Link node");

    let has_text = link_node
        .descendants()
        .any(|n| n.kind() == SyntaxKind::LinkText);
    let has_dest = link_node
        .descendants()
        .any(|n| n.kind() == SyntaxKind::LinkDestination);

    assert!(has_text, "Link should have LinkText");
    assert!(has_dest, "Link should have LinkDestination");
}

#[test]
fn test_parse_emoji_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("<:smile:12345678-1234-1234-1234-123456789abc>");
    let root = parsed.syntax();

    let has_emoji = root.descendants().any(|n| n.kind() == SyntaxKind::Emoji);

    assert!(has_emoji, "Should have Emoji node");
}

#[test]
fn test_parse_mention_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("<@12345678-1234-1234-1234-123456789abc>");
    let root = parsed.syntax();

    let has_mention = root.descendants().any(|n| n.kind() == SyntaxKind::Mention);

    assert!(has_mention, "Should have Mention node");
}

#[test]
fn test_nested_bold_italic_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("**bold *italic* more**");
    let root = parsed.syntax();

    let strong_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Strong)
        .expect("Should have Strong node");

    let has_emphasis = strong_node
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Emphasis);

    assert!(
        has_emphasis,
        "Strong should contain Emphasis (nested italic)"
    );
}

#[test]
fn test_link_with_bold_text_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("[**bold** link](https://example.com)");
    let root = parsed.syntax();

    let link_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Link)
        .expect("Should have Link node");

    let has_strong = link_node
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Strong);

    assert!(has_strong, "Link text should contain Strong (bold)");
}

#[test]
fn test_parse_spoiler_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("||spoiler text||");
    let root = parsed.syntax();

    let spoiler_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Spoiler)
        .expect("Should have Spoiler node");

    let delimiter_count = spoiler_node
        .children_with_tokens()
        .filter(|child| {
            if let NodeOrToken::Token(t) = child {
                t.kind() == SyntaxKind::SpoilerDelimiter
            } else {
                false
            }
        })
        .count();

    assert_eq!(delimiter_count, 2, "Should have two spoiler delimiters");
}

#[test]
fn test_spoiler_with_nested_bold_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("||**bold** inside||");
    let root = parsed.syntax();

    let spoiler_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Spoiler)
        .expect("Should have Spoiler node");

    let has_strong = spoiler_node
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Strong);

    assert!(has_strong, "Spoiler should contain Strong (bold)");
}

#[test]
fn test_spoiler_with_nested_italic_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("||*italic* inside||");
    let root = parsed.syntax();

    let spoiler_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Spoiler)
        .expect("Should have Spoiler node");

    let has_emphasis = spoiler_node
        .descendants()
        .any(|n| n.kind() == SyntaxKind::Emphasis);

    assert!(has_emphasis, "Spoiler should contain Emphasis (italic)");
}
