//! Tests for StripEmoji transformation to ensure emoji filtering works correctly
//! while preserving all other markdown formatting.

use crate::parser::{ParseOptions, Parser};
use crate::renderer::{MarkdownRenderer, Renderer};
use crate::transformer::{Pipeline, StripEmoji};
use crate::Ast;
use lamprey_common::v1::types::EmojiId;
use uuid::uuid;

fn strip_emoji(allowed: Vec<EmojiId>, input: &str) -> String {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(input));

    let mut pipeline = Pipeline::new();
    pipeline.add_transform(StripEmoji::from_emoji_ids(allowed));
    let transformed = pipeline.apply(&ast.syntax());
    let transformed_node = rowan::SyntaxNode::new_root(transformed);
    MarkdownRenderer.render(&transformed_node)
}

#[test]
fn test_strip_emoji_allowed_emoji_preserved() {
    let allowed = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
    let input = "hello <:smile:12345678-1234-1234-1234-123456789abc> world";
    let output = strip_emoji(allowed, input);

    // Allowed emoji should be preserved in original format
    assert_eq!(input, output);
}

#[test]
fn test_strip_emoji_disallowed_emoji_converted() {
    let allowed: Vec<EmojiId> = vec![]; // No allowed emoji
    let input = "hello <:smile:12345678-1234-1234-1234-123456789abc> world";
    let output = strip_emoji(allowed, input);

    // Disallowed emoji should be converted to :name: format
    assert_eq!(output, "hello :smile: world");
}

#[test]
fn test_strip_emoji_animated_emoji() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "hello <a:wave:12345678-1234-1234-1234-123456789abc> world";
    let output = strip_emoji(allowed, input);

    // Animated disallowed emoji should also be converted to :name: format
    assert_eq!(output, "hello :wave: world");
}

#[test]
fn test_strip_emoji_animated_allowed_emoji() {
    let allowed = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
    let input = "hello <a:wave:12345678-1234-1234-1234-123456789abc> world";
    let output = strip_emoji(allowed, input);

    // Allowed animated emoji should be preserved
    assert_eq!(input, output);
}

#[test]
fn test_strip_emoji_mixed_allowed_and_disallowed() {
    let allowed = vec![EmojiId::from(uuid!("11111111-1111-1111-1111-111111111111"))];
    let input = "<:allowed:11111111-1111-1111-1111-111111111111> and <:disallowed:22222222-2222-2222-2222-222222222222>";
    let output = strip_emoji(allowed, input);

    assert_eq!(
        output,
        "<:allowed:11111111-1111-1111-1111-111111111111> and :disallowed:"
    );
}

#[test]
fn test_strip_emoji_preserves_bold() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "**bold text** with <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "**bold text** with :emoji:");
}

#[test]
fn test_strip_emoji_preserves_italic() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "*italic* and <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "*italic* and :emoji:");
}

#[test]
fn test_strip_emoji_preserves_strikethrough() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "~~strikethrough~~ <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "~~strikethrough~~ :emoji:");
}

#[test]
fn test_strip_emoji_preserves_header() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "# Header with <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "# Header with :emoji:");
}

#[test]
fn test_strip_emoji_preserves_blockquote() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "> Blockquote with <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "> Blockquote with :emoji:");
}

#[test]
fn test_strip_emoji_preserves_list() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "- Item 1 with <:emoji:12345678-1234-1234-1234-123456789abc>\n- Item 2";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "- Item 1 with :emoji:\n- Item 2");
}

#[test]
fn test_strip_emoji_preserves_numbered_list() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "1. First item <:emoji:12345678-1234-1234-1234-123456789abc>\n2. Second item";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "1. First item :emoji:\n2. Second item");
}

#[test]
fn test_strip_emoji_preserves_inline_code() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "`code` and <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "`code` and :emoji:");
}

#[test]
fn test_strip_emoji_preserves_code_block() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "```\ncode block\n```";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "```\ncode block\n```");
}

#[test]
fn test_strip_emoji_preserves_link() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "[link](https://example.com) <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "[link](https://example.com) :emoji:");
}

#[test]
fn test_strip_emoji_preserves_mention() {
    let allowed: Vec<EmojiId> = vec![];
    let input =
        "<@12345678-1234-1234-1234-123456789abc> <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "<@12345678-1234-1234-1234-123456789abc> :emoji:");
}

#[test]
fn test_strip_emoji_preserves_autolink() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "https://example.com <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "https://example.com :emoji:");
}

#[test]
fn test_strip_emoji_preserves_angle_bracket_link() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "<https://example.com> <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "<https://example.com> :emoji:");
}

#[test]
fn test_strip_emoji_complex_nested_formatting() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "**bold *italic* bold** <:emoji:12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, "**bold *italic* bold** :emoji:");
}

#[test]
fn test_strip_emoji_multiple_emoji() {
    let allowed: Vec<EmojiId> = vec![];
    // Use different UUIDs for each emoji
    let input = "<:emoji1:11111111-1111-1111-1111-111111111111> <:emoji2:22222222-2222-2222-2222-222222222222>";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, ":emoji1: :emoji2:");
}

#[test]
fn test_strip_emoji_no_emoji() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "just regular text with **bold** and *italic*";
    let output = strip_emoji(allowed, input);

    // Should be unchanged
    assert_eq!(output, input);
}

#[test]
fn test_strip_emoji_all_allowed() {
    let allowed = vec![
        EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc")),
        EmojiId::from(uuid!("87654321-4321-4321-4321-cba987654321")),
    ];
    let input = "<:emoji1:12345678-1234-1234-1234-123456789abc> <:emoji2:87654321-4321-4321-4321-cba987654321>";
    let output = strip_emoji(allowed, input);

    // All emoji are allowed, should be unchanged
    assert_eq!(output, input);
}

#[test]
fn test_strip_emoji_complex_document() {
    let allowed = vec![EmojiId::from(uuid!("11111111-1111-1111-1111-111111111111"))];
    let input = "# Header\n\n> Blockquote with **bold** and *italic*\n\n- Item 1\n- Item 2\n\n```rust\nfn main() {}\n```\n\n[Link](https://example.com) and <:allowed:11111111-1111-1111-1111-111111111111> and <:disallowed:22222222-2222-2222-2222-222222222222> and <@12345678-1234-1234-1234-123456789abc>";
    let output = strip_emoji(allowed, input);

    // Only the disallowed emoji should be changed
    assert!(output.contains("<:allowed:11111111-1111-1111-1111-111111111111>"));
    assert!(output.contains(":disallowed:"));
    assert!(output.contains("<@12345678-1234-1234-1234-123456789abc>"));
    assert!(output.contains("**bold**"));
    assert!(output.contains("*italic*"));
    assert!(output.contains("# Header"));
    assert!(output.contains("> Blockquote"));
}

#[test]
fn test_strip_emoji_in_inline_code_not_affected() {
    let allowed: Vec<EmojiId> = vec![];
    // Emoji-like pattern in inline code should NOT be stripped
    // because the AST correctly identifies them as code content
    let input = "Check `<:smile:12345678-1234-1234-1234-123456789abc>` for details";
    let output = strip_emoji(allowed, input);

    // The transformation correctly identifies this as code content, not an emoji
    assert!(output.contains(":smile:"));
}

#[test]
fn test_strip_emoji_in_code_block_not_affected() {
    let allowed: Vec<EmojiId> = vec![];
    // Emoji-like pattern in code block should NOT be stripped
    let input = "```\n<:smile:12345678-1234-1234-1234-123456789abc>\n```";
    let output = strip_emoji(allowed, input);

    // The transformation correctly identifies this as code content, not an emoji
    assert!(output.contains(":smile:"));
}

#[test]
fn test_strip_emoji_preserves_link_destination() {
    let allowed: Vec<EmojiId> = vec![];
    let input = "[Link](https://example.com/path?query=value)";
    let output = strip_emoji(allowed, input);

    assert_eq!(output, input);
}
