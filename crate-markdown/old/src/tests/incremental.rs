//! Context-sensitive incremental parsing tests.

use crate::parser::{Edit, ParseOptions, Parser, SyntaxKind};
use rowan::TextRange;

#[test]
fn test_incremental_edit() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("**hello** world");

    let edit = Edit {
        delete: TextRange::new(10.into(), 15.into()),
        insert: "world!",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "**hello** world!");

    let root = edited.syntax();
    assert!(root.children().count() > 0);
}

#[test]
fn test_incremental_edit_reuses_tree() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("**hello** world");

    let original_strong_count = original
        .syntax()
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Strong)
        .count();

    let edit = Edit {
        delete: TextRange::new(10.into(), 15.into()),
        insert: "universe",
    };

    let edited = parser.edit(&original, edit);

    let edited_strong_count = edited
        .syntax()
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Strong)
        .count();

    assert_eq!(
        original_strong_count, edited_strong_count,
        "Bold should be preserved after edit"
    );
    assert_eq!(edited.source(), "**hello** universe");
}

#[test]
fn test_incremental_edit_breaks_emphasis() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("*foo*\nbar");

    let edit = Edit {
        delete: TextRange::new(1.into(), 1.into()),
        insert: "*",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "**foo*\nbar");

    let root = edited.syntax();
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    assert!(
        !has_emphasis,
        "Broken emphasis should not create Emphasis node"
    );
}

#[test]
fn test_incremental_edit_breaks_list() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("- item");

    let edit = Edit {
        delete: TextRange::new(0.into(), 2.into()),
        insert: "text ",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "text item");

    let root = edited.syntax();
    let has_list = root.descendants().any(|n| n.kind() == SyntaxKind::List);
    assert!(!has_list, "Removing dash should break the list");
}

#[test]
fn test_incremental_edit_fixes_emphasis() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("foo bar");

    let edit1 = Edit {
        delete: TextRange::new(0.into(), 0.into()),
        insert: "*",
    };
    let intermediate = parser.edit(&original, edit1);

    let edit2 = Edit {
        delete: TextRange::new(4.into(), 4.into()),
        insert: "*",
    };
    let edited = parser.edit(&intermediate, edit2);

    assert_eq!(edited.source(), "*foo* bar");

    let root = edited.syntax();
    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    assert!(has_emphasis, "Added delimiters should create Emphasis node");
}

#[test]
fn test_incremental_edit_paragraph_merges() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("first\n\nsecond");

    let edit = Edit {
        delete: TextRange::new(5.into(), 6.into()),
        insert: "",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "first\nsecond");

    let root = edited.syntax();
    let paragraph_count = root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::Paragraph)
        .count();
    assert_eq!(paragraph_count, 1, "Should have one merged paragraph");
}

#[test]
fn test_incremental_edit_code_block_fence() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("```code```");

    let edit = Edit {
        delete: TextRange::new(8.into(), 9.into()),
        insert: "",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "```code``");

    let root = edited.syntax();
    assert!(root.children().count() > 0, "Tree should be valid");
}

#[test]
fn test_incremental_edit_list_item_content() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("- *item*");

    let edit = Edit {
        delete: TextRange::new(3.into(), 3.into()),
        insert: "*",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "- **item*");

    let root = edited.syntax();
    let has_list = root.descendants().any(|n| n.kind() == SyntaxKind::List);
    assert!(has_list, "List should still exist");

    let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
    assert!(!has_emphasis, "Broken emphasis should not exist");
}

#[test]
fn test_incremental_edit_blockquote_marker() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("> quote");

    let edit = Edit {
        delete: TextRange::new(0.into(), 2.into()),
        insert: "text ",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "text quote");

    let root = edited.syntax();
    let has_blockquote = root
        .descendants()
        .any(|n| n.kind() == SyntaxKind::BlockQuote);
    assert!(!has_blockquote, "Removing > should break blockquote");
}

#[test]
fn test_incremental_edit_header_marker() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("# Header");

    let edit = Edit {
        delete: TextRange::new(0.into(), 2.into()),
        insert: "text ",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "text Header");

    let root = edited.syntax();
    let has_header = root.descendants().any(|n| n.kind() == SyntaxKind::Header);
    assert!(!has_header, "Removing # should break header");
}

#[test]
fn test_incremental_edit_multiple_blocks_affected() {
    let parser = Parser::new(ParseOptions::default());
    let original = parser.parse("- item\n\nparagraph");

    let edit = Edit {
        delete: TextRange::new(7.into(), 8.into()),
        insert: "",
    };

    let edited = parser.edit(&original, edit);
    assert_eq!(edited.source(), "- item\nparagraph");

    let root = edited.syntax();
    assert!(root.children().count() > 0, "Tree should be valid");
}
