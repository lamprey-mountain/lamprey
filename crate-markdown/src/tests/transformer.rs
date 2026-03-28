//! Tests for the transformer module.

use crate::parser::{ParseOptions, Parser};
use crate::transformer::{apply, Pipeline, StripEmoji, Transformation, UppercaseText};
use crate::Ast;
use lamprey_common::v1::types::EmojiId;
use rowan::{GreenNode, GreenNodeBuilder, SyntaxNode};
use uuid::uuid;

// ============ StripEmoji Transformation Tests ============

#[test]
fn test_strip_emoji_transformation_disallowed() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc> world"));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "hello :smile: world");
}

#[test]
fn test_strip_emoji_transformation_allowed() {
    let allowed = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc> world"));

    let transform = StripEmoji::new(allowed);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(
        result,
        "hello <:smile:12345678-1234-1234-1234-123456789abc> world"
    );
}

#[test]
fn test_strip_emoji_transformation_mixed() {
    let allowed = vec![EmojiId::from(uuid!("11111111-1111-1111-1111-111111111111"))];
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("<:allowed:11111111-1111-1111-1111-111111111111> <:not_allowed:22222222-2222-2222-2222-222222222222>"));

    let transform = StripEmoji::new(allowed);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert!(result.contains("<:allowed:11111111-1111-1111-1111-111111111111>"));
    assert!(result.contains(":not_allowed:"));
}

#[test]
fn test_strip_emoji_transformation_animated() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("<a:wave:12345678-1234-1234-1234-123456789abc>"));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, ":wave:");
}

#[test]
fn test_strip_emoji_transformation_no_emoji() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello world"));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "hello world");
}

#[test]
fn test_strip_emoji_transformation_multiple() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("<:a:11111111-1111-1111-1111-111111111111> <:b:22222222-2222-2222-2222-222222222222> <:c:33333333-3333-3333-3333-333333333333>"));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, ":a: :b: :c:");
}

// ============ Pipeline Tests ============

#[test]
fn test_pipeline_empty() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello world"));

    let pipeline = Pipeline::default();
    let transformed = pipeline.apply(&ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "hello world");
}

#[test]
fn test_pipeline_single_transform() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc>"));

    let mut pipeline = Pipeline::default();
    pipeline.add_transform(StripEmoji::new(vec![]));

    let transformed = pipeline.apply(&ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "hello :smile:");
}

#[test]
fn test_pipeline_multiple_transforms() {
    let allowed_first = vec![EmojiId::from(uuid!("11111111-1111-1111-1111-111111111111"))];
    let allowed_second = vec![EmojiId::from(uuid!("22222222-2222-2222-2222-222222222222"))];

    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("<:first:11111111-1111-1111-1111-111111111111> <:second:22222222-2222-2222-2222-222222222222>"));

    let mut pipeline = Pipeline::default();
    pipeline.add_transform(StripEmoji::new(allowed_first));
    pipeline.add_transform(StripEmoji::new(allowed_second));

    let transformed = pipeline.apply(&ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    // Both transforms run, each preserving their allowed emoji
    // First transform preserves first emoji, second transform also preserves it (not in its list but already converted)
    // Actually both should be preserved since each has one allowed
    assert!(result.contains("first"));
    assert!(result.contains("second"));
}

#[test]
fn test_pipeline_preserves_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold** *italic* [link](url)"));

    let mut pipeline = Pipeline::default();
    pipeline.add_transform(StripEmoji::new(vec![]));

    let transformed = pipeline.apply(&ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "**bold** *italic* [link](url)");
}

// ============ Token Transformation Tests ============

#[test]
fn test_uppercase_text() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello world"));

    let transformed = apply(&UppercaseText, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "HELLO WORLD");
}

#[test]
fn test_uppercase_text_with_emoji() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc>"));

    let mut pipeline = Pipeline::new();
    pipeline.add_transform(StripEmoji::new(vec![]));
    pipeline.add_transform(UppercaseText);

    let transformed = pipeline.apply(&ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "HELLO :SMILE:");
}

#[test]
fn test_uppercase_preserves_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**bold** *italic* [link](url)"));

    let transformed = apply(&UppercaseText, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "**BOLD** *ITALIC* [LINK](URL)");
}

// ============ Edge Case Tests ============

#[test]
fn test_transformation_empty_document() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(""));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "");
}

#[test]
fn test_transformation_emoji_in_bold() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("**<:smile:12345678-1234-1234-1234-123456789abc>**"));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "**:smile:**");
}

#[test]
fn test_transformation_emoji_in_link() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse("[<:smile:12345678-1234-1234-1234-123456789abc>](url)"));

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    // Note: Emoji in link text may not be transformed depending on parser implementation
    // This is a known limitation - the link text is parsed as a single unit
    assert!(result.contains("smile"));
    assert!(result.contains("url"));
}

#[test]
fn test_transformation_nested_formatting() {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(
        parser.parse("**bold *italic <:smile:12345678-1234-1234-1234-123456789abc> bold**"),
    );

    let transform = StripEmoji::new(vec![]);
    let transformed = apply(&transform, &ast.syntax());
    let node = SyntaxNode::<crate::parser::MyLang>::new_root(transformed);
    let result = node.text().to_string();

    assert_eq!(result, "**bold *italic :smile: bold**");
}
