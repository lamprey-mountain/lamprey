//! Basic formatting tests.

use crate::ast::Ast;
use crate::parser::{ParseOptions, Parser, SyntaxKind};
use rowan::{NodeOrToken, SyntaxNode};

fn parse(source: &str) -> SyntaxNode<crate::parser::MyLang> {
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(source);
    parsed.syntax()
}

fn collect_kinds(node: &SyntaxNode<crate::parser::MyLang>) -> Vec<SyntaxKind> {
    node.descendants_with_tokens()
        .filter_map(|el| match el {
            NodeOrToken::Node(n) => Some(n.kind()),
            NodeOrToken::Token(t) => Some(t.kind()),
        })
        .collect()
}

#[test]
fn test_plain_text() {
    let root = parse("hello world");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Text));
    assert!(kinds.contains(&SyntaxKind::Paragraph));
}

#[test]
fn test_bold() {
    let root = parse("**hello**");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Strong));
    assert!(kinds.contains(&SyntaxKind::StrongDelimiter));
}

#[test]
fn test_bold_with_text_around() {
    let root = parse("before **bold** after");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Strong));
    assert!(kinds.contains(&SyntaxKind::StrongDelimiter));
    assert!(kinds.contains(&SyntaxKind::Text));
}

#[test]
fn test_italic() {
    let root = parse("*hello*");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Emphasis));
    assert!(kinds.contains(&SyntaxKind::EmphasisDelimiter));
}

#[test]
fn test_italic_with_text_around() {
    let root = parse("before *italic* after");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Emphasis));
    assert!(kinds.contains(&SyntaxKind::EmphasisDelimiter));
    assert!(kinds.contains(&SyntaxKind::Text));
}

#[test]
fn test_bold_and_italic() {
    let root = parse("**bold** and *italic*");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Strong));
    assert!(kinds.contains(&SyntaxKind::Emphasis));
}

#[test]
fn test_nested_bold_italic() {
    let root = parse("**bold *italic* more**");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Strong));
    assert!(kinds.contains(&SyntaxKind::Emphasis));
}

#[test]
fn test_unmatched_bold() {
    let root = parse("**unmatched");
    let kinds = collect_kinds(&root);
    assert!(!kinds.contains(&SyntaxKind::Strong));
    assert!(kinds.contains(&SyntaxKind::Text));
}

#[test]
fn test_unmatched_italic() {
    let root = parse("*unmatched");
    let kinds = collect_kinds(&root);
    assert!(!kinds.contains(&SyntaxKind::Emphasis));
    assert!(kinds.contains(&SyntaxKind::Text));
}

#[test]
fn test_empty_bold() {
    let root = parse("****");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Strong));
}

#[test]
fn test_single_star() {
    let root = parse("*");
    let kinds = collect_kinds(&root);
    assert!(!kinds.contains(&SyntaxKind::Emphasis));
    assert!(kinds.contains(&SyntaxKind::Text));
}

#[test]
fn test_multiple_paragraphs() {
    let root = parse("first\n\nsecond");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Paragraph));
    assert!(kinds.contains(&SyntaxKind::Text));
}

#[test]
fn test_bold_in_multiple_paragraphs() {
    let root = parse("**first**\n\n**second**");
    let kinds = collect_kinds(&root);
    let strong_count = kinds.iter().filter(|&&k| k == SyntaxKind::Strong).count();
    assert_eq!(
        strong_count, 2,
        "Expected 2 Strong nodes, got {}",
        strong_count
    );
}

#[test]
fn test_plain_url() {
    let root = parse("check https://example.com out");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Autolink));
    assert!(kinds.contains(&SyntaxKind::LinkDestination));
}

#[test]
fn test_named_link() {
    let root = parse("[example](https://example.com)");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Link));
    assert!(kinds.contains(&SyntaxKind::LinkText));
    assert!(kinds.contains(&SyntaxKind::LinkDestination));
}

#[test]
fn test_angle_bracket_link() {
    let root = parse("<https://example.com>");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::AngleBracketLink));
}

#[test]
fn test_link_with_parentheses() {
    let root = parse("[Stromboli](https://en.wikipedia.org/wiki/Stromboli_(disambiguation))");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Link));
    assert!(kinds.contains(&SyntaxKind::LinkDestination));
    let dests: Vec<_> = kinds
        .iter()
        .filter(|&&k| k == SyntaxKind::LinkDestination)
        .collect();
    assert!(!dests.is_empty());
}

#[test]
fn test_link_with_bold_text() {
    let root = parse("[**bold** link](https://example.com)");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Link));
    assert!(kinds.contains(&SyntaxKind::Strong));
}

#[test]
fn test_multiple_links() {
    let root = parse("[first](https://a.com) and [second](https://b.com)");
    let kinds = collect_kinds(&root);
    let link_count = kinds.iter().filter(|&&k| k == SyntaxKind::Link).count();
    assert_eq!(link_count, 2, "Expected 2 Link nodes");
}

#[test]
fn test_unmatched_bracket() {
    let root = parse("[no closing bracket");
    let kinds = collect_kinds(&root);
    assert!(!kinds.contains(&SyntaxKind::Link));
}

#[test]
fn test_bracket_without_url() {
    let root = parse("[text] no url");
    let kinds = collect_kinds(&root);
    assert!(!kinds.contains(&SyntaxKind::Link));
}

#[test]
fn test_identity_reader() {
    use crate::render::IdentityReader;

    let source = "**hello** *world* https://example.com";
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(source);
    let ast = Ast::new(parsed);
    let reader = IdentityReader;
    let result = reader.read(&ast);
    assert_eq!(result, source);
}

#[test]
fn test_strikethrough() {
    let root = parse("~~deleted~~");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Strikethrough));
    assert!(kinds.contains(&SyntaxKind::StrikethroughDelimiter));
}

#[test]
fn test_inline_code() {
    let root = parse("`code`");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::InlineCode));
    assert!(kinds.contains(&SyntaxKind::InlineCodeFence));
    assert!(kinds.contains(&SyntaxKind::InlineCodeContent));
}

#[test]
fn test_mention() {
    let root = parse("@12345678-1234-1234-1234-123456789abc");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Mention));
}

#[test]
fn test_role_mention() {
    let root = parse("<@&12345678-1234-1234-1234-123456789abc>");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::MentionRole));
    assert!(kinds.contains(&SyntaxKind::MentionMarker));
}

#[test]
fn test_channel_mention() {
    let root = parse("<#12345678-1234-1234-1234-123456789abc>");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::MentionChannel));
    assert!(kinds.contains(&SyntaxKind::MentionMarker));
}

#[test]
fn test_emoji() {
    let root = parse("<:smile:12345678-1234-1234-1234-123456789abc>");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Emoji));
    assert!(kinds.contains(&SyntaxKind::EmojiName));
}

#[test]
fn test_animated_emoji() {
    let root = parse("<a:wave:12345678-1234-1234-1234-123456789abc>");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Emoji));
    assert!(kinds.contains(&SyntaxKind::EmojiName));
    let source = root.text().to_string();
    assert!(source.starts_with("<a:"));
}

#[test]
fn test_header() {
    let root = parse("# Header 1");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::Header));
    assert!(kinds.contains(&SyntaxKind::HeaderMarker));
}

#[test]
fn test_bullet_list() {
    let root = parse("- item 1\n- item 2");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::List));
    assert!(kinds.contains(&SyntaxKind::ListItem));
    assert!(kinds.contains(&SyntaxKind::ListMarker));
}

#[test]
fn test_numbered_list() {
    let root = parse("1. first\n2. second");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::List));
    assert!(kinds.contains(&SyntaxKind::ListItem));
}

#[test]
fn test_blockquote() {
    let root = parse("> quoted text");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::BlockQuote));
    assert!(kinds.contains(&SyntaxKind::BlockQuoteMarker));
}

#[test]
fn test_code_block() {
    let root = parse("```code```");
    let kinds = collect_kinds(&root);
    assert!(kinds.contains(&SyntaxKind::CodeBlock));
    assert!(kinds.contains(&SyntaxKind::CodeBlockFence));
}

#[test]
fn test_plain_text_reader() {
    use crate::render::PlainTextReader;

    let source = "**hello** *world* ~~deleted~~";
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(source);
    let ast = Ast::new(parsed);
    let reader = PlainTextReader::new();
    let result = reader.read(&ast);
    assert!(result.contains("hello"));
    assert!(result.contains("world"));
    assert!(result.contains("deleted"));
}

#[test]
fn test_strip_emoji_reader() {
    use crate::render::StripEmojiReader;
    use lamprey_common::v1::types::EmojiId;
    use uuid::uuid;

    let allowed_emoji = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
    let source = "hello <:smile:12345678-1234-1234-1234-123456789abc> world";
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(source);
    let ast = Ast::new(parsed);
    let reader = StripEmojiReader::new(allowed_emoji);
    let result = reader.read(&ast);
    assert!(
        result.contains("hello") || result.contains("world"),
        "Should contain text"
    );
    assert!(
        result.contains("<:smile:12345678-1234-1234-1234-123456789abc>"),
        "Allowed emoji should preserve UUID"
    );
}

#[test]
fn test_strip_emoji_reader_filters_disallowed() {
    use crate::render::StripEmojiReader;
    use lamprey_common::v1::types::EmojiId;
    use uuid::uuid;

    let allowed_emoji: Vec<EmojiId> = vec![];
    let source = "hello <:smile:12345678-1234-1234-1234-123456789abc> world";
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(source);
    let ast = Ast::new(parsed);
    let reader = StripEmojiReader::new(allowed_emoji);
    let result = reader.read(&ast);

    assert!(
        result.contains("hello") || result.contains("world"),
        "Should contain text"
    );
    assert!(result.contains(":smile:"), "Should contain emoji name");
    assert!(!result.contains("12345678-1234-1234-1234-123456789abc"));
}

#[test]
fn test_strip_emoji_reader_mixed() {
    use crate::render::StripEmojiReader;
    use lamprey_common::v1::types::EmojiId;
    use uuid::uuid;

    let allowed_emoji = vec![EmojiId::from(uuid!("11111111-1111-1111-1111-111111111111"))];
    let source =
        "<:allowed:11111111-1111-1111-1111-111111111111> <:disallowed:22222222-2222-2222-2222-222222222222>";
    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(source);
    let ast = Ast::new(parsed);
    let reader = StripEmojiReader::new(allowed_emoji);
    let result = reader.read(&ast);

    assert!(
        result.contains(":allowed:"),
        "Should contain allowed emoji name"
    );
    assert!(
        result.contains("11111111-1111-1111-1111-111111111111"),
        "Allowed emoji should preserve UUID"
    );
    assert!(
        result.contains(":disallowed:"),
        "Disallowed emoji should be :name: format"
    );
    assert!(
        !result.contains("22222222-2222-2222-2222-222222222222"),
        "Disallowed emoji UUID should not be in output"
    );
}

#[test]
fn test_role_mention_in_code_ignored() {
    let root = parse("check `<@&12345678-1234-1234-1234-123456789abc>` out");
    // Role mention inside inline code should not be parsed as MentionRole
    let has_role_mention = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::MentionRole);
    assert!(
        !has_role_mention,
        "Role mention inside code should not create MentionRole node"
    );
}

#[test]
fn test_channel_mention_in_code_ignored() {
    let root = parse("check `<#12345678-1234-1234-1234-123456789abc>` out");
    // Channel mention inside inline code should not be parsed as MentionChannel
    let has_channel_mention = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::MentionChannel);
    assert!(
        !has_channel_mention,
        "Channel mention inside code should not create MentionChannel node"
    );
}

#[test]
fn test_role_mention_inside_bold() {
    let root = parse("**<@&12345678-1234-1234-1234-123456789abc>**");
    let has_role = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::MentionRole);
    let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);

    assert!(has_role, "Should have role mention inside bold");
    assert!(has_strong, "Should have strong/bold");
}

#[test]
fn test_channel_mention_inside_italic() {
    let root = parse("*<#12345678-1234-1234-1234-123456789abc>*");
    let has_channel = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::MentionChannel);
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

    assert!(has_channel, "Should have channel mention inside italic");
    assert!(has_emphasis, "Should have emphasis/italic");
}
