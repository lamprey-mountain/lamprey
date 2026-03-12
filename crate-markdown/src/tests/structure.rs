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

    assert_eq!(delimiter_count, 2, "Should have two delimiters");
}

#[test]
fn test_parse_strikethrough_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("~~deleted~~");
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

    assert_eq!(delimiter_count, 2, "Should have two delimiters");
}

#[test]
fn test_parse_inline_code_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("`code`");
    let root = parsed.syntax();

    let code_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::InlineCode)
        .expect("Should have InlineCode node");

    let fence_count = code_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::InlineCodeFence)
        .count();
    let has_content = code_node
        .children()
        .any(|child| child.kind() == SyntaxKind::InlineCodeContent);

    assert_eq!(fence_count, 2, "Should have two fences");
    assert!(has_content, "Should have content node");
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
        .children()
        .any(|child| child.kind() == SyntaxKind::LinkText);
    let has_dest = link_node
        .children()
        .any(|child| child.kind() == SyntaxKind::LinkDestination);

    assert!(has_text, "Should have LinkText");
    assert!(has_dest, "Should have LinkDestination");
}

#[test]
fn test_parse_mention_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("@12345678-1234-1234-1234-123456789abc");
    let root = parsed.syntax();

    let mention_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Mention)
        .expect("Should have Mention node");

    let has_marker = mention_node.children_with_tokens().any(|child| {
        if let NodeOrToken::Token(t) = child {
            t.kind() == SyntaxKind::MentionMarker
        } else {
            false
        }
    });

    assert!(has_marker, "Should have MentionMarker");
}

#[test]
fn test_parse_emoji_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("<:smile:12345678-1234-1234-1234-123456789abc>");
    let root = parsed.syntax();

    let emoji_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Emoji)
        .expect("Should have Emoji node");

    let marker_count = emoji_node
        .children_with_tokens()
        .filter(|child| {
            if let NodeOrToken::Token(t) = child {
                t.kind() == SyntaxKind::EmojiMarker
            } else {
                false
            }
        })
        .count();
    let has_name = emoji_node
        .children()
        .any(|child| child.kind() == SyntaxKind::EmojiName);

    assert!(marker_count >= 2, "Should have at least 2 EmojiMarkers");
    assert!(has_name, "Should have EmojiName");
}

#[test]
fn test_parse_header_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("# Header");
    let root = parsed.syntax();

    let header_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::Header)
        .expect("Should have Header node");

    let has_marker = header_node
        .children()
        .any(|child| child.kind() == SyntaxKind::HeaderMarker);

    assert!(has_marker, "Should have HeaderMarker");
}

#[test]
fn test_parse_list_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("- item");
    let root = parsed.syntax();

    let list_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::List)
        .expect("Should have List node");

    let list_item = list_node
        .children()
        .find(|n| n.kind() == SyntaxKind::ListItem)
        .expect("Should have ListItem");

    let has_marker = list_item
        .children()
        .any(|child| child.kind() == SyntaxKind::ListMarker);

    assert!(has_marker, "Should have ListMarker");
}

#[test]
fn test_parse_blockquote_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("> quote");
    let root = parsed.syntax();

    let quote_node = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::BlockQuote)
        .expect("Should have BlockQuote node");

    let has_marker = quote_node
        .children()
        .any(|n| n.kind() == SyntaxKind::BlockQuoteMarker);

    assert!(has_marker, "Should have BlockQuoteMarker");
}

#[test]
fn test_parse_code_block_structure() {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse("```code```");
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
