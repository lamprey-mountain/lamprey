use crate::parser::Parser;

#[test]
fn test_ordered_list() {
    let source = "1. foo\n2. bar\n3. baz";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<ol><li>foo</li><li>bar</li><li>baz</li></ol>"
    );
    assert_eq!(parsed.to_plain(), "1. foo\n2. bar\n3. baz");
}

#[test]
fn test_unordered_list_dash() {
    let source = "- foo\n- bar\n- baz";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<ul><li>foo</li><li>bar</li><li>baz</li></ul>"
    );
    assert_eq!(parsed.to_plain(), "- foo\n- bar\n- baz");
}

#[test]
fn test_unordered_list_star() {
    let source = "* foo\n* bar\n* baz";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        "<ul><li>foo</li><li>bar</li><li>baz</li></ul>"
    );
    assert_eq!(parsed.to_plain(), "- foo\n- bar\n- baz");
}

#[test]
fn test_task_list() {
    let source = "- [ ] foo\n- [x] bar";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    assert_eq!(
        parsed.to_html(),
        r#"<ul class="task-list"><li class="task-item"><input class="task-checkbox" type="checkbox"  disabled />foo</li><li class="task-item"><input class="task-checkbox" type="checkbox" checked disabled />bar</li></ul>"#
    );
    assert_eq!(parsed.to_plain(), "- [ ] foo\n- [x] bar");
}

// TODO: add more tests
// - test malformed whitespace
// - test nested lists
// - test block formatting inside lists
