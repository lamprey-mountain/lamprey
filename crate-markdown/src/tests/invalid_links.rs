use crate::ast::block::Block;
use crate::ast::block::Document;
use crate::ast::inline::Inline;
use crate::parser::Parser;
use crate::prelude::*;

#[test]
fn test_link_panic() {
    let source = "[link]";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    let doc = Document::cast(parsed.tree_ref().root().clone()).unwrap();
    for block in doc.children() {
        if let Block::Paragraph(p) = block {
            for inline in p.children() {
                if let Inline::Link(l) = inline {
                    println!("Found link, href: {}", l.href());
                }
            }
        }
    }
}

#[test]
fn test_incomplete_link_panic() {
    let source = "[link](";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    let doc = Document::cast(parsed.tree_ref().root().clone()).unwrap();
    for block in doc.children() {
        if let Block::Paragraph(p) = block {
            for inline in p.children() {
                if let Inline::Link(l) = inline {
                    println!("Found link, href: {}", l.href());
                }
            }
        }
    }
}

#[test]
fn test_empty_link_panic() {
    let source = "[link]()";
    let parser = Parser::new();
    let parsed = parser.parse(source);

    let doc = Document::cast(parsed.tree_ref().root().clone()).unwrap();
    for block in doc.children() {
        if let Block::Paragraph(p) = block {
            for inline in p.children() {
                if let Inline::Link(l) = inline {
                    println!("Found link, href: {}", l.href());
                }
            }
        }
    }
}
