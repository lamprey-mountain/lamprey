use crate::parser::Parser;

#[test]
fn test_edit_insertion() {
    let source = "hello world";
    let parser = Parser::new();
    let mut parsed = parser.parse(source);

    // Insert "beautiful " at index 6
    parsed.edit((6, 6).into(), "beautiful ");

    assert_eq!(parsed.to_markdown(), "hello beautiful world");
    assert_eq!(parsed.to_html(), "<p>hello beautiful world</p>");
}

#[test]
fn test_edit_deletion() {
    let source = "hello beautiful world";
    let parser = Parser::new();
    let mut parsed = parser.parse(source);

    // Delete "beautiful " (indices 6 to 16)
    parsed.edit((6, 16).into(), "");

    assert_eq!(parsed.to_markdown(), "hello world");
    assert_eq!(parsed.to_html(), "<p>hello world</p>");
}

#[test]
fn test_edit_replacement() {
    let source = "hello world";
    let parser = Parser::new();
    let mut parsed = parser.parse(source);

    // Replace "world" (indices 6 to 11) with "friend"
    parsed.edit((6, 11).into(), "friend");

    assert_eq!(parsed.to_markdown(), "hello friend");
    assert_eq!(parsed.to_html(), "<p>hello friend</p>");
}

#[test]
fn test_edit_inline_formatting() {
    let source = "hello *world*";
    let parser = Parser::new();
    let mut parsed = parser.parse(source);

    // Replace "world" (indices 7 to 12) with "friend"
    parsed.edit((7, 12).into(), "friend");

    assert_eq!(parsed.to_markdown(), "hello *friend*");
    assert_eq!(parsed.to_html(), "<p>hello <em>friend</em></p>");
}

#[test]
fn test_edit_outside_of_bounds() {
    let source = "hello beautiful world";
    let parser = Parser::new();
    let mut parsed = parser.parse(source);

    // Replace everything starting from "beautiful world"
    parsed.edit((6, 9999).into(), "friend");

    assert_eq!(parsed.to_markdown(), "hello friend");
    assert_eq!(parsed.to_html(), "<p>hello friend</p>");
}
