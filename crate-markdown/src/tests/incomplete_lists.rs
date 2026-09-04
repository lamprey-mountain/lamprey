use crate::parser::Parser;

#[test]
fn test_ordered_list_incomplete() {
    let source = "1. a\n2.";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<ol><li>a</li><li></li></ol>");
    assert_eq!(parsed.to_plain(), "1. a\n2.");
}

#[test]
fn test_unordered_list_incomplete() {
    let source = "- a\n-";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(parsed.to_html(), "<ul><li>a</li><li></li></ul>");
    assert_eq!(parsed.to_plain(), "- a\n-");
}

#[test]
fn test_task_list_incomplete() {
    let source = "- [ ] a\n- [ ]";
    let parser = Parser::new();
    let parsed = parser.parse(source);
    assert_eq!(
        parsed.to_html(),
        r#"<ul class="task-list"><li class="task-item"><input class="task-checkbox" type="checkbox"  disabled />a</li><li class="task-item"><input class="task-checkbox" type="checkbox"  disabled /></li></ul>"#
    );
    assert_eq!(parsed.to_plain(), "- [ ] a\n- [ ]");
}
