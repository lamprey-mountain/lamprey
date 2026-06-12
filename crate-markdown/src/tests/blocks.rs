use crate::parser::Parser;

#[test]
fn test_headers() {
    let source = "# h1\n## h2\n###### h6";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<h1>h1</h1>\n<h2>h2</h2>\n<h6>h6</h6>"
    );
}

#[test]
fn test_codeblocks() {
    let source = "```rust\nfn main() {}\n```\n```\nno lang\n```";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<pre><code class=\"language-rust\">fn main() {}</code></pre><p></p><pre><code class=\"language-text\">no lang</code></pre>"
    );
}

#[test]
fn test_quotes() {
    let source = "> quote\n> line 2\n\n> another quote";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<blockquote>quote\nline 2\n</blockquote>\n<blockquote>another quote</blockquote>"
    );
}
