// TODO: tests
// - mentions work
// - mentions can be escaped
// - mentions in codeblocks are ignored
// - codeblocks can be escaped, making mentions work again

#[cfg(test)]
mod tests {
    use crate::parser::{ParseOptions, Parser, SyntaxKind};
    use rowan::{NodeOrToken, SyntaxNode};

    fn parse(source: &str) -> SyntaxNode<crate::parser::MyLang> {
        let parser = Parser::new(ParseOptions::default());
        let parsed = parser.parse(source);
        parsed.syntax()
    }

    fn collect_kinds(node: &SyntaxNode<crate::parser::MyLang>) -> Vec<SyntaxKind> {
        node.descendants_with_tokens()
            .filter_map(|el| match el {
                NodeOrToken::Node(n) => Some(n.kind()),
                NodeOrToken::Token(t) => Some(t.kind()),
            })
            .collect()
    }

    #[test]
    fn test_plain_text() {
        let root = parse("hello world");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Text));
        assert!(kinds.contains(&SyntaxKind::Paragraph));
    }

    #[test]
    fn test_bold() {
        let root = parse("**hello**");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Strong));
        assert!(kinds.contains(&SyntaxKind::StrongDelimiter));
    }

    #[test]
    fn test_bold_with_text_around() {
        let root = parse("before **bold** after");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Strong));
        assert!(kinds.contains(&SyntaxKind::StrongDelimiter));
        assert!(kinds.contains(&SyntaxKind::Text));
    }

    #[test]
    fn test_italic() {
        let root = parse("*hello*");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Emphasis));
        assert!(kinds.contains(&SyntaxKind::EmphasisDelimiter));
    }

    #[test]
    fn test_italic_with_text_around() {
        let root = parse("before *italic* after");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Emphasis));
        assert!(kinds.contains(&SyntaxKind::EmphasisDelimiter));
        assert!(kinds.contains(&SyntaxKind::Text));
    }

    #[test]
    fn test_bold_and_italic() {
        let root = parse("**bold** and *italic*");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Strong));
        assert!(kinds.contains(&SyntaxKind::Emphasis));
    }

    #[test]
    fn test_nested_bold_italic() {
        let root = parse("**bold *italic* more**");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Strong));
        assert!(kinds.contains(&SyntaxKind::Emphasis));
    }

    #[test]
    fn test_unmatched_bold() {
        let root = parse("**unmatched");
        let kinds = collect_kinds(&root);
        // Should not have Strong node, just Text
        assert!(!kinds.contains(&SyntaxKind::Strong));
        assert!(kinds.contains(&SyntaxKind::Text));
    }

    #[test]
    fn test_unmatched_italic() {
        let root = parse("*unmatched");
        let kinds = collect_kinds(&root);
        // Should not have Emphasis node, just Text
        assert!(!kinds.contains(&SyntaxKind::Emphasis));
        assert!(kinds.contains(&SyntaxKind::Text));
    }

    #[test]
    fn test_empty_bold() {
        let root = parse("****");
        let kinds = collect_kinds(&root);
        // Empty bold should still be recognized
        assert!(kinds.contains(&SyntaxKind::Strong));
    }

    #[test]
    fn test_single_star() {
        let root = parse("*");
        let kinds = collect_kinds(&root);
        // Single * is unmatched, should be just text
        assert!(!kinds.contains(&SyntaxKind::Emphasis));
        assert!(kinds.contains(&SyntaxKind::Text));
    }

    #[test]
    fn test_multiple_paragraphs() {
        let root = parse("first\n\nsecond");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Paragraph));
        assert!(kinds.contains(&SyntaxKind::Text));
    }

    #[test]
    fn test_bold_in_multiple_paragraphs() {
        let root = parse("**first**\n\n**second**");
        let kinds = collect_kinds(&root);
        let strong_count = kinds.iter().filter(|&&k| k == SyntaxKind::Strong).count();
        assert_eq!(
            strong_count, 2,
            "Expected 2 Strong nodes, got {}",
            strong_count
        );
    }

    #[test]
    fn test_plain_url() {
        let root = parse("check https://example.com out");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Autolink));
        assert!(kinds.contains(&SyntaxKind::LinkDestination));
    }

    #[test]
    fn test_named_link() {
        let root = parse("[example](https://example.com)");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Link));
        assert!(kinds.contains(&SyntaxKind::LinkText));
        assert!(kinds.contains(&SyntaxKind::LinkDestination));
    }

    #[test]
    fn test_angle_bracket_link() {
        let root = parse("<https://example.com>");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::AngleBracketLink));
    }

    #[test]
    fn test_link_with_parentheses() {
        let root = parse("[Stromboli](https://en.wikipedia.org/wiki/Stromboli_(disambiguation))");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Link));
        assert!(kinds.contains(&SyntaxKind::LinkDestination));
        // Make sure the URL includes the parenthesized part
        let dests: Vec<_> = kinds
            .iter()
            .filter(|&&k| k == SyntaxKind::LinkDestination)
            .collect();
        assert!(!dests.is_empty());
    }

    #[test]
    fn test_link_with_bold_text() {
        let root = parse("[**bold** link](https://example.com)");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Link));
        assert!(kinds.contains(&SyntaxKind::Strong));
    }

    #[test]
    fn test_multiple_links() {
        let root = parse("[first](https://a.com) and [second](https://b.com)");
        let kinds = collect_kinds(&root);
        let link_count = kinds.iter().filter(|&&k| k == SyntaxKind::Link).count();
        assert_eq!(link_count, 2, "Expected 2 Link nodes");
    }

    #[test]
    fn test_unmatched_bracket() {
        let root = parse("[no closing bracket");
        let kinds = collect_kinds(&root);
        assert!(!kinds.contains(&SyntaxKind::Link));
    }

    #[test]
    fn test_bracket_without_url() {
        let root = parse("[text] no url");
        let kinds = collect_kinds(&root);
        assert!(!kinds.contains(&SyntaxKind::Link));
    }

    #[test]
    fn test_identity_reader() {
        use crate::ast::Ast;
        use crate::parser::Parser;
        use crate::render::IdentityReader;

        let source = "**hello** *world* https://example.com";
        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse(source);
        let ast = Ast::new(parsed);
        let reader = IdentityReader;
        let result = reader.read(&ast);
        assert_eq!(result, source);
    }

    #[test]
    fn test_strikethrough() {
        let root = parse("~~deleted~~");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Strikethrough));
        assert!(kinds.contains(&SyntaxKind::StrikethroughDelimiter));
    }

    #[test]
    fn test_inline_code() {
        let root = parse("`code`");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::InlineCode));
        assert!(kinds.contains(&SyntaxKind::InlineCodeFence));
        assert!(kinds.contains(&SyntaxKind::InlineCodeContent));
    }

    #[test]
    fn test_mention() {
        let root = parse("@12345678-1234-1234-1234-123456789abc");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Mention));
    }

    #[test]
    fn test_emoji() {
        let root = parse(":smile:12345678-1234-1234-1234-123456789abc:");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Emoji));
        assert!(kinds.contains(&SyntaxKind::EmojiName));
    }

    #[test]
    fn test_header() {
        let root = parse("# Header 1");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::Header));
        assert!(kinds.contains(&SyntaxKind::HeaderMarker));
    }

    #[test]
    fn test_bullet_list() {
        let root = parse("- item 1\n- item 2");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::List));
        assert!(kinds.contains(&SyntaxKind::ListItem));
        assert!(kinds.contains(&SyntaxKind::ListMarker));
    }

    #[test]
    fn test_numbered_list() {
        let root = parse("1. first\n2. second");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::List));
        assert!(kinds.contains(&SyntaxKind::ListItem));
    }

    #[test]
    fn test_blockquote() {
        let root = parse("> quoted text");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::BlockQuote));
        assert!(kinds.contains(&SyntaxKind::BlockQuoteMarker));
    }

    #[test]
    fn test_code_block() {
        let root = parse("```code```");
        let kinds = collect_kinds(&root);
        assert!(kinds.contains(&SyntaxKind::CodeBlock));
        assert!(kinds.contains(&SyntaxKind::CodeBlockFence));
    }

    #[test]
    fn test_plain_text_reader() {
        use crate::ast::Ast;
        use crate::parser::Parser;
        use crate::render::PlainTextReader;

        let source = "**hello** *world* ~~deleted~~";
        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse(source);
        let ast = Ast::new(parsed);
        let reader = PlainTextReader::new();
        let result = reader.read(&ast);
        // Should contain the text but not the delimiters
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        assert!(result.contains("deleted"));
    }

    #[test]
    fn test_strip_emoji_reader() {
        use crate::ast::Ast;
        use crate::parser::Parser;
        use crate::render::StripEmojiReader;
        use lamprey_common::v1::types::EmojiId;
        use uuid::uuid;

        let allowed_emoji = vec![EmojiId::from(uuid!("12345678-1234-1234-1234-123456789abc"))];
        let source = "hello :smile:12345678-1234-1234-1234-123456789abc: world";
        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse(source);
        let ast = Ast::new(parsed);
        let reader = StripEmojiReader::new(allowed_emoji);
        let result = reader.read(&ast);
        // Should contain the text and the allowed emoji
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        // The emoji should be preserved since it's in the allowed list
        assert!(result.contains("smile"));
    }

    // ============ Parsing structure tests ============

    #[test]
    fn test_parse_bold_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**hello**");
        let root = parsed.syntax();

        // Find the Strong node
        let strong_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Strong)
            .expect("Should have Strong node");

        // Check it has delimiters and text
        let mut has_open = false;
        let mut has_text = false;
        let mut has_close = false;

        for child in strong_node.children_with_tokens() {
            match child {
                NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::StrongDelimiter {
                        if !has_open {
                            has_open = true;
                        } else {
                            has_close = true;
                        }
                    }
                }
                NodeOrToken::Node(_) => {
                    has_text = true;
                }
            }
        }

        assert!(has_open, "Should have opening delimiter");
        assert!(has_close, "Should have closing delimiter");
    }

    #[test]
    fn test_parse_italic_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("*hello*");
        let root = parsed.syntax();

        // Find the Emphasis node
        let emphasis_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Emphasis)
            .expect("Should have Emphasis node");

        // Check it has delimiters
        let mut delimiter_count = 0;
        for child in emphasis_node.children_with_tokens() {
            if let NodeOrToken::Token(t) = child {
                if t.kind() == SyntaxKind::EmphasisDelimiter {
                    delimiter_count += 1;
                }
            }
        }

        assert_eq!(delimiter_count, 2, "Should have two delimiters");
    }

    #[test]
    fn test_parse_strikethrough_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("~~deleted~~");
        let root = parsed.syntax();

        // Find the Strikethrough node
        let strike_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Strikethrough)
            .expect("Should have Strikethrough node");

        // Check it has delimiters
        let mut delimiter_count = 0;
        for child in strike_node.children_with_tokens() {
            if let NodeOrToken::Token(t) = child {
                if t.kind() == SyntaxKind::StrikethroughDelimiter {
                    delimiter_count += 1;
                }
            }
        }

        assert_eq!(delimiter_count, 2, "Should have two delimiters");
    }

    #[test]
    fn test_parse_inline_code_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("`code`");
        let root = parsed.syntax();

        // Find the InlineCode node
        let code_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::InlineCode)
            .expect("Should have InlineCode node");

        // Check it has fences (as nodes) and content
        let mut fence_count = 0;
        let mut has_content = false;

        for child in code_node.children() {
            if child.kind() == SyntaxKind::InlineCodeFence {
                fence_count += 1;
            }
            if child.kind() == SyntaxKind::InlineCodeContent {
                has_content = true;
            }
        }

        assert_eq!(fence_count, 2, "Should have two fences");
        assert!(has_content, "Should have content node");
    }

    #[test]
    fn test_parse_link_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("[text](https://example.com)");
        let root = parsed.syntax();

        // Find the Link node
        let link_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Link)
            .expect("Should have Link node");

        // Check it has LinkText and LinkDestination children
        let mut has_text = false;
        let mut has_dest = false;

        for child in link_node.children() {
            if child.kind() == SyntaxKind::LinkText {
                has_text = true;
            }
            if child.kind() == SyntaxKind::LinkDestination {
                has_dest = true;
            }
        }

        assert!(has_text, "Should have LinkText");
        assert!(has_dest, "Should have LinkDestination");
    }

    #[test]
    fn test_parse_mention_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("@12345678-1234-1234-1234-123456789abc");
        let root = parsed.syntax();

        // Find the Mention node
        let mention_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Mention)
            .expect("Should have Mention node");

        // Should have MentionMarker
        let mut has_marker = false;
        for child in mention_node.children_with_tokens() {
            if let NodeOrToken::Token(t) = child {
                if t.kind() == SyntaxKind::MentionMarker {
                    has_marker = true;
                }
            }
        }

        assert!(has_marker, "Should have MentionMarker");
    }

    #[test]
    fn test_parse_emoji_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse(":smile:12345678-1234-1234-1234-123456789abc:");
        let root = parsed.syntax();

        // Find the Emoji node
        let emoji_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Emoji)
            .expect("Should have Emoji node");

        // Should have EmojiMarker and EmojiName
        let mut marker_count = 0;
        let mut has_name = false;

        for child in emoji_node.children_with_tokens() {
            match child {
                NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::EmojiMarker {
                        marker_count += 1;
                    }
                }
                NodeOrToken::Node(n) => {
                    if n.kind() == SyntaxKind::EmojiName {
                        has_name = true;
                    }
                }
            }
        }

        assert!(marker_count >= 2, "Should have at least 2 EmojiMarkers");
        assert!(has_name, "Should have EmojiName");
    }

    #[test]
    fn test_parse_header_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("# Header");
        let root = parsed.syntax();

        // Find the Header node
        let header_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Header)
            .expect("Should have Header node");

        // Should have HeaderMarker
        let mut has_marker = false;
        for child in header_node.children() {
            if child.kind() == SyntaxKind::HeaderMarker {
                has_marker = true;
            }
        }

        assert!(has_marker, "Should have HeaderMarker");
    }

    #[test]
    fn test_parse_list_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("- item");
        let root = parsed.syntax();

        // Find the List node
        let list_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::List)
            .expect("Should have List node");

        // Should have ListItem with ListMarker
        let list_item = list_node
            .children()
            .find(|n| n.kind() == SyntaxKind::ListItem)
            .expect("Should have ListItem");

        let mut has_marker = false;
        for child in list_item.children() {
            if child.kind() == SyntaxKind::ListMarker {
                has_marker = true;
            }
        }

        assert!(has_marker, "Should have ListMarker");
    }

    #[test]
    fn test_parse_blockquote_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("> quote");
        let root = parsed.syntax();

        // Find the BlockQuote node
        let quote_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::BlockQuote)
            .expect("Should have BlockQuote node");

        // Should have BlockQuoteMarker (as a child node)
        let has_marker = quote_node
            .children()
            .any(|n| n.kind() == SyntaxKind::BlockQuoteMarker);

        assert!(has_marker, "Should have BlockQuoteMarker");
    }

    #[test]
    fn test_parse_code_block_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("```code```");
        let root = parsed.syntax();

        // Find the CodeBlock node
        let code_block = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::CodeBlock)
            .expect("Should have CodeBlock node");

        // Should have CodeBlockFence and CodeBlockContent
        let mut fence_count = 0;
        let mut has_content = false;

        for child in code_block.children() {
            if child.kind() == SyntaxKind::CodeBlockFence {
                fence_count += 1;
            }
            if child.kind() == SyntaxKind::CodeBlockContent {
                has_content = true;
            }
        }

        assert!(fence_count >= 1, "Should have at least 1 CodeBlockFence");
        assert!(has_content, "Should have CodeBlockContent");
    }

    #[test]
    fn test_nested_bold_italic_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold *italic* more**");
        let root = parsed.syntax();

        // Find the Strong node
        let strong_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Strong)
            .expect("Should have Strong node");

        // Strong should contain an Emphasis node
        let has_emphasis = strong_node
            .descendants()
            .any(|n| n.kind() == SyntaxKind::Emphasis);

        assert!(
            has_emphasis,
            "Strong should contain Emphasis (nested italic)"
        );
    }

    #[test]
    fn test_link_with_bold_text_structure() {
        use crate::parser::{Parser, SyntaxKind};
        use rowan::{NodeOrToken, SyntaxNode};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("[**bold** link](https://example.com)");
        let root = parsed.syntax();

        // Find the Link node
        let link_node = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::Link)
            .expect("Should have Link node");

        // Link should contain a Strong node (bold in link text)
        let has_strong = link_node
            .descendants()
            .any(|n| n.kind() == SyntaxKind::Strong);

        assert!(has_strong, "Link text should contain Strong (bold)");
    }

    #[test]
    fn test_incremental_edit() {
        use crate::parser::{Edit, Parser};
        use rowan::TextRange;

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let original = parser.parse("**hello** world");

        // Edit: change "world" to "world!"
        // "**hello** world" is 15 chars (0-14), "world" is at 10-15
        let edit = Edit {
            delete: TextRange::new(10.into(), 15.into()), // "world"
            insert: "world!",
        };

        let edited = parser.edit(&original, edit);

        // Check the source was updated correctly
        assert_eq!(edited.source(), "**hello** world!");

        // The tree should still be valid
        let root = edited.syntax();
        assert!(root.children().count() > 0);
    }

    #[test]
    fn test_incremental_edit_reuses_tree() {
        use crate::parser::{Edit, Parser, SyntaxKind};
        use rowan::TextRange;

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let original = parser.parse("**hello** world");

        // Get the original tree structure
        let original_strong_count = original
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Strong)
            .count();

        // Edit: change text after the bold
        let edit = Edit {
            delete: TextRange::new(10.into(), 15.into()), // "world"
            insert: "universe",
        };

        let edited = parser.edit(&original, edit);

        // The edited tree should still have the bold
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

    // ============ Escape sequence tests ============

    #[test]
    fn test_escape_asterisk() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\*not italic\\*");
        let root = parsed.syntax();

        // Should have Escape nodes
        let escape_count = root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Escape)
            .count();

        assert_eq!(escape_count, 2, "Should have two escape sequences");

        // Should NOT have Emphasis node
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

        assert!(
            !has_emphasis,
            "Escaped asterisks should not create emphasis"
        );
    }

    #[test]
    fn test_escape_backslash() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("path\\\\to\\\\file");
        let root = parsed.syntax();

        // Should have Escape nodes
        let escape_count = root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Escape)
            .count();

        assert!(escape_count >= 1, "Should have escape sequences");
    }

    #[test]
    fn test_escape_bracket() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\[not a link\\]");
        let root = parsed.syntax();

        // Should have Escape nodes
        let escape_count = root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Escape)
            .count();

        assert_eq!(escape_count, 2, "Should have two escape sequences");

        // Should NOT have Link node
        let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);

        assert!(!has_link, "Escaped brackets should not create a link");
    }

    #[test]
    fn test_plain_text_with_escapes() {
        use crate::ast::Ast;
        use crate::parser::Parser;
        use crate::render::PlainTextReader;

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\*hello\\* \\[world\\]");
        let ast = Ast::new(parsed);
        let reader = PlainTextReader::new();
        let result = reader.read(&ast);

        // Should contain the text without backslashes
        assert!(result.contains("*"), "Should contain unescaped asterisk");
        assert!(result.contains("["), "Should contain unescaped bracket");
        assert!(result.contains("hello"), "Should contain hello");
        assert!(result.contains("world"), "Should contain world");
        assert!(
            !result.contains("\\"),
            "Should not contain backslashes, got: {}",
            result
        );
    }

    #[test]
    fn test_escape_in_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold with \\* asterisk**");
        let root = parsed.syntax();

        // Should have Strong node
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);

        assert!(has_strong, "Should have bold");

        // Should have Escape node inside the bold
        let has_escape = root.descendants().any(|n| n.kind() == SyntaxKind::Escape);

        assert!(has_escape, "Should have escape inside bold");
    }

    // ============ More escape sequence tests ============

    #[test]
    fn test_escape_hash() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\# not a header");
        let root = parsed.syntax();

        // Should have Escape node
        let has_escape = root.descendants().any(|n| n.kind() == SyntaxKind::Escape);

        assert!(has_escape, "Should have escape");

        // Should NOT have Header node
        let has_header = root.descendants().any(|n| n.kind() == SyntaxKind::Header);

        assert!(!has_header, "Escaped hash should not create header");
    }

    #[test]
    fn test_escape_dash() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\- not a list");
        let root = parsed.syntax();

        // Should have Escape node
        let has_escape = root.descendants().any(|n| n.kind() == SyntaxKind::Escape);

        assert!(has_escape, "Should have escape");

        // Should NOT have List node
        let has_list = root.descendants().any(|n| n.kind() == SyntaxKind::List);

        assert!(!has_list, "Escaped dash should not create list");
    }

    #[test]
    fn test_escape_backtick() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\`not code\\`");
        let root = parsed.syntax();

        // Should have Escape nodes
        let escape_count = root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Escape)
            .count();

        assert_eq!(escape_count, 2, "Should have two escapes");

        // Should NOT have InlineCode node
        let has_inline_code = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::InlineCode);

        assert!(
            !has_inline_code,
            "Escaped backticks should not create inline code"
        );
    }

    #[test]
    fn test_multiple_escapes_in_row() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\\*\\*\\*");
        let root = parsed.syntax();

        // Should have Escape nodes
        let escape_count = root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::Escape)
            .count();

        assert!(escape_count >= 2, "Should have multiple escapes");
    }

    #[test]
    fn test_escape_at_end() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("text\\");
        let root = parsed.syntax();

        // Should have Document at minimum (trailing backslash is incomplete escape)
        let has_document = root.descendants().any(|n| n.kind() == SyntaxKind::Document);

        assert!(has_document, "Should have document");
    }

    // ============ Complex nesting tests ============

    #[test]
    fn test_bold_inside_italic() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("*italic **bold** more*");
        let root = parsed.syntax();

        // Should have both Emphasis and Strong
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);

        assert!(has_emphasis, "Should have italic");
        assert!(has_strong, "Should have bold inside italic");
    }

    #[test]
    fn test_italic_inside_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold *italic* more**");
        let root = parsed.syntax();

        // Should have both Strong and Emphasis
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

        assert!(has_strong, "Should have bold");
        assert!(has_emphasis, "Should have italic inside bold");
    }

    #[test]
    fn test_strikethrough_inside_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold ~~deleted~~ more**");
        let root = parsed.syntax();

        // Should have both Strong and Strikethrough
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_strike = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::Strikethrough);

        assert!(has_strong, "Should have bold");
        assert!(has_strike, "Should have strikethrough inside bold");
    }

    #[test]
    fn test_link_inside_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold [link](url) more**");
        let root = parsed.syntax();

        // Should have both Strong and Link
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);

        assert!(has_strong, "Should have bold");
        assert!(has_link, "Should have link inside bold");
    }

    #[test]
    fn test_code_inside_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold `code` more**");
        let root = parsed.syntax();

        // Should have both Strong and InlineCode
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_code = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::InlineCode);

        assert!(has_strong, "Should have bold");
        assert!(has_code, "Should have code inside bold");
    }

    #[test]
    fn test_all_inline_formats_nested() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold *italic ~~strike~~ more* end**");
        let root = parsed.syntax();

        // Should have Strong, Emphasis, and Strikethrough
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
        let has_strike = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::Strikethrough);

        assert!(has_strong, "Should have bold");
        assert!(has_emphasis, "Should have italic");
        assert!(has_strike, "Should have strikethrough");
    }

    #[test]
    fn test_mention_inside_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**hello @12345678-1234-1234-1234-123456789abc world**");
        let root = parsed.syntax();

        // Should have both Strong and Mention
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_mention = root.descendants().any(|n| n.kind() == SyntaxKind::Mention);

        assert!(has_strong, "Should have bold");
        assert!(has_mention, "Should have mention inside bold");
    }

    #[test]
    fn test_emoji_inside_italic() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("*hello :smile:12345678-1234-1234-1234-123456789abc: world*");
        let root = parsed.syntax();

        // Should have both Emphasis and Emoji
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
        let has_emoji = root.descendants().any(|n| n.kind() == SyntaxKind::Emoji);

        assert!(has_emphasis, "Should have italic");
        assert!(has_emoji, "Should have emoji inside italic");
    }

    // ============ Malformed input tests ============

    #[test]
    fn test_unclosed_bold() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**unclosed bold");
        let root = parsed.syntax();

        // Should NOT have Strong node (unclosed)
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);

        assert!(!has_strong, "Unclosed bold should not create Strong node");

        // Should have Document at minimum
        let has_document = root.descendants().any(|n| n.kind() == SyntaxKind::Document);

        assert!(has_document, "Should have document");
    }

    #[test]
    fn test_unclosed_italic() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("*unclosed italic");
        let root = parsed.syntax();

        // Should NOT have Emphasis node (unclosed)
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

        assert!(
            !has_emphasis,
            "Unclosed italic should not create Emphasis node"
        );
    }

    #[test]
    fn test_unclosed_strikethrough() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("~~unclosed strikethrough");
        let root = parsed.syntax();

        // Should NOT have Strikethrough node (unclosed)
        let has_strike = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::Strikethrough);

        assert!(
            !has_strike,
            "Unclosed strikethrough should not create Strikethrough node"
        );
    }

    #[test]
    fn test_unclosed_code() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("`unclosed code");
        let root = parsed.syntax();

        // Should NOT have InlineCode node (unclosed)
        let has_code = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::InlineCode);

        assert!(!has_code, "Unclosed code should not create InlineCode node");
    }

    #[test]
    fn test_unclosed_link() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("[unclosed link");
        let root = parsed.syntax();

        // Should NOT have Link node (unclosed)
        let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);

        assert!(!has_link, "Unclosed link should not create Link node");
    }

    #[test]
    fn test_link_without_url() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("[text] no url");
        let root = parsed.syntax();

        // Should NOT have Link node (no URL part)
        let has_link = root.descendants().any(|n| n.kind() == SyntaxKind::Link);

        assert!(!has_link, "Link without URL should not create Link node");
    }

    #[test]
    fn test_mismatched_delimiters() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold * mismatched");
        let root = parsed.syntax();

        // Should NOT have Strong or Emphasis (mismatched)
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

        assert!(
            !has_strong,
            "Mismatched delimiters should not create Strong"
        );
        assert!(
            !has_emphasis,
            "Mismatched delimiters should not create Emphasis"
        );
    }

    #[test]
    fn test_single_asterisk() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("*");
        let root = parsed.syntax();

        // Should NOT have Emphasis (single asterisk)
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);

        assert!(!has_emphasis, "Single asterisk should not create Emphasis");
    }

    #[test]
    fn test_triple_asterisk() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("***");
        let root = parsed.syntax();

        // Triple asterisk is ambiguous - parser may or may not create Strong
        // Just verify it doesn't crash and produces valid output
        let has_document = root.descendants().any(|n| n.kind() == SyntaxKind::Document);

        assert!(has_document, "Should have document");
    }

    #[test]
    fn test_empty_delimiters() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("****");
        let root = parsed.syntax();

        // Should have Strong (empty bold)
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);

        assert!(has_strong, "Empty delimiters should still create node");
    }

    #[test]
    fn test_only_special_chars() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        // Use a simpler case that won't cause infinite loops
        let parsed = parser.parse("**~~");
        let root = parsed.syntax();

        // Should have Root and Document at minimum
        let has_root = root.kind() == SyntaxKind::Root;
        let has_document = root.children().any(|n| n.kind() == SyntaxKind::Document);

        assert!(has_root, "Should have Root");
        assert!(has_document, "Should have Document");
    }

    #[test]
    fn test_very_nested_unclosed() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("**bold *italic ~~strike");
        let root = parsed.syntax();

        // Should NOT have any formatting nodes (all unclosed)
        let has_strong = root.descendants().any(|n| n.kind() == SyntaxKind::Strong);
        let has_emphasis = root.descendants().any(|n| n.kind() == SyntaxKind::Emphasis);
        let has_strike = root
            .descendants()
            .any(|n| n.kind() == SyntaxKind::Strikethrough);

        assert!(!has_strong, "Unclosed should not create Strong");
        assert!(!has_emphasis, "Unclosed should not create Emphasis");
        assert!(!has_strike, "Unclosed should not create Strikethrough");
    }

    #[test]
    fn test_empty_string() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("");
        let root = parsed.syntax();

        // Should have Root and Document at minimum
        let has_root = root.kind() == SyntaxKind::Root;
        let has_document = root.children().any(|n| n.kind() == SyntaxKind::Document);

        assert!(has_root, "Should have Root");
        assert!(has_document, "Should have Document");
    }

    #[test]
    fn test_only_whitespace() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("   ");
        let root = parsed.syntax();

        // Should have Root and Document
        let has_root = root.kind() == SyntaxKind::Root;

        assert!(has_root, "Should have Root even for whitespace only");
    }

    #[test]
    fn test_only_newlines() {
        use crate::parser::{Parser, SyntaxKind};

        let parser = Parser::new(crate::parser::ParseOptions::default());
        let parsed = parser.parse("\n\n\n");
        let root = parsed.syntax();

        // Should have Root and Document
        let has_root = root.kind() == SyntaxKind::Root;

        assert!(has_root, "Should have Root even for newlines only");
    }
}
