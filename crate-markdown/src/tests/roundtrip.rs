//! Round-trip / Idempotency tests to ensure text fidelity preservation.

use crate::parser::{ParseOptions, Parser};
use crate::{Ast, IdentityReader};

#[test]
fn test_markdown_roundtrip_header() {
    let inputs = vec![
        "# Header",
        "## Header 2",
        "### Header 3 with trailing space ",
        "###### Header 6",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_blockquote() {
    let inputs = vec![
        "> Blockquote",
        "> > Nested blockquote",
        "> Multiple\n> lines",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_list() {
    let inputs = vec![
        "- Bullet list",
        "* Another bullet",
        "+ Plus bullet",
        "1. Numbered list",
        "2. Second item",
        "- Item 1\n- Item 2",
        "1. First\n2. Second\n3. Third",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_code_block() {
    let inputs = vec![
        "```rust\ncode here\n```",
        "```\nplain code\n```",
        "```\n\nindented\n\n```",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_strong() {
    let inputs = vec!["**bold**", "**bold text**", "Multiple **bold** words"];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_emphasis() {
    let inputs = vec!["*italic*", "*italic text*", "Multiple *italic* words"];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_strikethrough() {
    let inputs = vec![
        "~~strikethrough~~",
        "~~strikethrough text~~",
        "Multiple ~~strikethrough~~ words",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_inline_code() {
    let inputs = vec!["`code`", "`inline code`", "Multiple `code` words"];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_link() {
    let inputs = vec![
        "[link text](https://example.com)",
        "[multiple](http://test.com) links",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_mention() {
    let inputs = vec![
        "<@12345678-1234-1234-1234-123456789abc>",
        "Hello <@12345678-1234-1234-1234-123456789abc> world",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_emoji() {
    let inputs = vec![
        "<:smile:12345678-1234-1234-1234-123456789abc>",
        "<a:wave:12345678-1234-1234-1234-123456789abc>",
        "Hello <:smile:12345678-1234-1234-1234-123456789abc> world",
    ];

    let parser = Parser::new(ParseOptions::default());

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let reader = IdentityReader;
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_markdown_roundtrip_complex() {
    let input = "# Header\n\n> Blockquote with **bold** and *italic*\n\n- Item 1\n- Item 2\n\n```rust\nfn main() {}\n```\n\n[Link](https://example.com) and <:smile:12345678-1234-1234-1234-123456789abc> and <@12345678-1234-1234-1234-123456789abc>";

    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(input));
    let reader = IdentityReader;
    let output = reader.read(&ast);

    assert_eq!(input, output);
}

#[test]
fn test_markdown_roundtrip_escaped() {
    let input = "Escaped \\*asterisk\\* and \\\\ backslash";

    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(input));
    let reader = IdentityReader;
    let output = reader.read(&ast);

    assert_eq!(input, output);
}

#[test]
fn test_identity_reader_preserves_source() {
    let inputs = vec![
        "**hello** world",
        "# Header\n> quote",
        "- list\n- item",
        "```rust\ncode\n```",
        "<:smile:12345678-1234-1234-1234-123456789abc>",
    ];

    let parser = Parser::new(ParseOptions::default());
    use crate::render::IdentityReader;
    let reader = IdentityReader;

    for input in inputs {
        let ast = Ast::new(parser.parse(input));
        let output = reader.read(&ast);
        assert_eq!(input, output);
    }
}

#[test]
fn test_cst_contains_all_tokens() {
    let input = "**bold** `code` *italic* ~~strike~~";

    let parser = Parser::new(ParseOptions::default());
    let parsed = parser.parse(input);
    let root = parsed.syntax();

    let text: String = root
        .descendants_with_tokens()
        .filter_map(|it| it.into_token())
        .map(|t| t.text().to_string())
        .collect();

    assert_eq!(text, input, "CST should contain all original tokens");
}
