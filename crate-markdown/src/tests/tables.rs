use crate::parser::Parser;

#[test]
fn test_basic_table() {
    let source = "| a | b |\n|---|---|\n| 1 | 2 |";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>"
    );
}

#[test]
fn test_table_with_formatting() {
    let source = "| **bold** | *italic* |\n|---|---|\n| `code` | ~~strike~~ |";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<table><thead><tr><th><strong>bold</strong></th><th><em>italic</em></th></tr></thead><tbody><tr><td><code>code</code></td><td><s>strike</s></td></tr></tbody></table>"
    );
}
