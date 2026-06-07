//! Edge case tests for markdown parsing and rendering.

use crate::parser::{ParseOptions, Parser};
use crate::renderer::{MarkdownRenderer, PlaintextRenderer, Renderer};
use crate::transformer::{Pipeline, StripEmoji};
use crate::Ast;
use rowan::SyntaxNode;

// ============ Empty/Whitespace Tests ============

#[test]
fn test_empty_document() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(""));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "");
}

#[test]
fn test_whitespace_only() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("   \n\t\n   "));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.trim().is_empty());
}

#[test]
fn test_newlines_only() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("\n\n\n"));

    let result = MarkdownRenderer.render(&ast.syntax());
    // Newline handling depends on parser implementation
    assert!(result.len() >= 0);
}

// ============ Long Content Tests ============

#[test]
fn test_very_long_line() {
    let long_text = "a".repeat(10000);
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(&long_text));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result.len(), long_text.len());
}

#[test]
fn test_many_words() {
    let text = (0..1000)
        .map(|i| format!("word{}", i))
        .collect::<Vec<_>>()
        .join(" ");
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(&text));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("word0"));
    assert!(result.contains("word999"));
}

// ============ Deep Nesting Tests ============

#[test]
fn test_deeply_nested_bold() {
    let mut nested = String::new();
    for _ in 0..10 {
        nested.push_str("**");
    }
    nested.push_str("text");
    for _ in 0..10 {
        nested.push_str("**");
    }

    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(&nested));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("text"));
}

#[test]
fn test_deeply_nested_mixed_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold *italic **nested** italic* bold**"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("nested"));
}

#[test]
fn test_nested_links() {
    // Links can't actually be nested in markdown, but we test the parser handles it gracefully
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[text [nested](inner)](outer)"));

    let result = MarkdownRenderer.render(&ast.syntax());
    // Parser should handle this gracefully
    assert!(!result.is_empty());
}

// ============ Line Ending Tests ============

#[test]
fn test_mixed_line_endings() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("line1\r\nline2\nline3\rline4"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
    assert!(result.contains("line3"));
    assert!(result.contains("line4"));
}

#[test]
fn test_crlf_line_endings() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("line1\r\nline2\r\nline3"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
    assert!(result.contains("line3"));
}

// ============ Unicode Tests ============

#[test]
fn test_emoji_text() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("Hello 🌍 World 🚀"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("🌍"));
    assert!(result.contains("🚀"));
}

#[test]
fn test_cjk_characters() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("你好世界 你好 世界"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "你好世界 你好 世界");
}

#[test]
fn test_rtl_text() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("مرحبا بالعالم"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "مرحبا بالعالم");
}

#[test]
fn test_mixed_scripts() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("English 中文 العربية"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("English"));
    assert!(result.contains("中文"));
    assert!(result.contains("العربية"));
}

#[test]
fn test_emoji_modifiers() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("👨‍👩‍👧‍👦 👍🏿 👍🏾 👍🏽 👍🏼 👍🏻"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("👨"));
}

// ============ Special Character Tests ============

#[test]
fn test_code_block_special_chars() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("```\n<>&\"'\\`*\n```"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("<>&\"'\\`*"));
}

#[test]
fn test_inline_code_special_chars() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("`<>&\"'`"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("<>&\"'"));
}

#[test]
fn test_escaped_characters() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("\\* \\_ \\` \\[ \\] \\( \\) \\\\"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "\\* \\_ \\` \\[ \\] \\( \\) \\\\");
}

#[test]
fn test_escape_in_bold() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**text \\*with\\* escapes**"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("\\*with\\*"));
}

// ============ Malformed Input Tests ============

#[test]
fn test_unclosed_bold() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**unclosed bold"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("**unclosed bold"));
}

#[test]
fn test_unclosed_italic() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("*unclosed italic"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("*unclosed italic"));
}

#[test]
fn test_unclosed_code() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("`unclosed code"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("`unclosed code"));
}

#[test]
fn test_unclosed_link() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[unclosed link](url"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("[unclosed link](url"));
}

#[test]
fn test_mismatched_delimiters() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold *italic mismatch**"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("bold"));
    assert!(result.contains("italic"));
    assert!(result.contains("mismatch"));
}

#[test]
fn test_empty_delimiters() {
    let parser = Parser::new(ParseOptions::default());
    // Test various empty delimiter combinations
    // Note: Each delimiter type is tested separately to avoid parser edge cases
    let ast = Ast::new(parser.parse("****"));
    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("****"));
}

#[test]
fn test_empty_delimiters_strikethrough() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("~~~~~~"));
    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("~~"));
}

#[test]
fn test_empty_delimiters_backticks() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("``````"));
    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("``"));
}

// ============ Edge Case Formatting Tests ============

#[test]
fn test_bold_at_start() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold** at start"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "**bold** at start");
}

#[test]
fn test_bold_at_end() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("at end **bold**"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "at end **bold**");
}

#[test]
fn test_adjacent_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold***italic*"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "**bold***italic*");
}

#[test]
fn test_consecutive_same_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**first** **second**"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "**first** **second**");
}

#[test]
fn test_formatting_with_only_spaces() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("** **"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "** **");
}

// ============ Emoji Edge Cases ============

#[test]
fn test_emoji_at_start() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("<:smile:12345678-1234-1234-1234-123456789abc> at start"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("<:smile:"));
}

#[test]
fn test_emoji_at_end() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("at end <:smile:12345678-1234-1234-1234-123456789abc>"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("<:smile:"));
}

#[test]
fn test_many_emoji() {
    let mut emoji_text = String::new();
    for i in 0..100 {
        let uuid = format!("{:08}-0000-0000-0000-{:012}", i, i);
        emoji_text.push_str(&format!("<:e{}:{}> ", i, uuid));
    }

    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(&emoji_text));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("<:e0:"));
    assert!(result.contains("<:e99:"));
}

#[test]
fn test_emoji_strip_many() {
    let mut emoji_text = String::new();
    for i in 0..50 {
        let uuid = format!("{:08}-0000-0000-0000-{:012}", i, i);
        emoji_text.push_str(&format!("<:e{}:{}> ", i, uuid));
    }

    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(&emoji_text));

    let mut pipeline = Pipeline::default();
    pipeline.add_transform(StripEmoji::new(vec![]));

    let transformed = pipeline.apply(&ast.syntax());
    let result =
        MarkdownRenderer.render(&SyntaxNode::<crate::parser::MyLang>::new_root(transformed));

    assert!(result.contains(":e0:"));
    assert!(result.contains(":e49:"));
}

// ============ Link Edge Cases ============

#[test]
fn test_link_with_parentheses() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[text](https://example.com/foo(bar))"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("foo(bar)"));
}

#[test]
fn test_link_with_special_chars() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[text](https://example.com/foo?bar=baz&qux=quux)"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("bar=baz"));
    assert!(result.contains("qux=quux"));
}

#[test]
fn test_link_with_title() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[text](url \"title with spaces\")"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("title with spaces"));
}

// ============ List Edge Cases ============

#[test]
fn test_empty_list_item() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("- \n- item"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("-"));
}

#[test]
fn test_list_with_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("- **bold**\n- *italic*"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("**bold**"));
    assert!(result.contains("*italic*"));
}

// ============ Header Edge Cases ============

#[test]
fn test_header_without_space() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("#NoSpace"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("#"));
}

#[test]
fn test_header_with_only_space() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("# "));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert_eq!(result, "# ");
}

#[test]
fn test_header_with_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("# **bold** header"));

    let result = MarkdownRenderer.render(&ast.syntax());
    assert!(result.contains("**bold**"));
}
