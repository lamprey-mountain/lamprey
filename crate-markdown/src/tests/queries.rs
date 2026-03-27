//! AST Query tests.

use crate::ast::{Ast, LinkKind, MentionId, MentionIds};
use crate::parser::Parser;
use uuid::uuid;

#[test]
fn test_extract_links_raw_url() {
    let parser = Parser::default();
    let ast = Ast::new(parser.parse("check https://example.com out"));

    let links: Vec<_> = ast.links().collect();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, LinkKind::RawUrl);
    assert!(links[0].dest.contains("example.com"));
}

#[test]
fn test_extract_links_angle_bracket() {
    let parser = Parser::default();
    let ast = Ast::new(parser.parse("check <https://example.com> out"));

    let links: Vec<_> = ast.links().collect();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, LinkKind::AngleBracket);
    assert_eq!(links[0].dest, "https://example.com");
}

#[test]
fn test_extract_links_named() {
    let parser = Parser::default();
    let ast = Ast::new(parser.parse("check [example](https://example.com) out"));

    let links: Vec<_> = ast.links().collect();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, LinkKind::Named);
    assert_eq!(links[0].dest, "https://example.com");
    assert_eq!(links[0].text, Some(std::borrow::Cow::Borrowed("example")));
}

#[test]
fn test_extract_links_mixed() {
    let parser = Parser::default();
    let ast =
        Ast::new(parser.parse("see https://a.com and [b](https://b.com) and <https://c.com>"));

    let links: Vec<_> = ast.links().collect();
    assert_eq!(links.len(), 3);
}

#[test]
fn test_extract_mentions_user() {
    let parser = Parser::default();
    let expected_uuid = uuid!("12345678-1234-1234-1234-123456789abc");
    let ast = Ast::new(parser.parse("hello <@12345678-1234-1234-1234-123456789abc>"));

    let mentions: Vec<_> = ast.mentions().collect();
    assert_eq!(mentions.len(), 1);
    match &mentions[0] {
        MentionId::User(uuid) => assert_eq!(**uuid, expected_uuid),
        _ => panic!("Expected User mention"),
    }
}

#[test]
fn test_extract_mentions_emoji() {
    let parser = Parser::default();
    let expected_uuid = uuid!("12345678-1234-1234-1234-123456789abc");
    let ast = Ast::new(parser.parse("hello <:smile:12345678-1234-1234-1234-123456789abc>"));

    let mentions: Vec<_> = ast.mentions().collect();
    assert_eq!(mentions.len(), 1);
    match &mentions[0] {
        MentionId::Emoji { id, name, animated } => {
            assert_eq!(**id, expected_uuid);
            assert_eq!(name, "smile");
            assert!(!animated);
        }
        _ => panic!("Expected Emoji mention"),
    }
}

#[test]
fn test_extract_mentions_animated_emoji() {
    let parser = Parser::default();
    let expected_uuid = uuid!("12345678-1234-1234-1234-123456789abc");
    let ast = Ast::new(parser.parse("hello <a:wave:12345678-1234-1234-1234-123456789abc>"));

    let mentions: Vec<_> = ast.mentions().collect();
    assert_eq!(mentions.len(), 1);
    match &mentions[0] {
        MentionId::Emoji { id, name, animated } => {
            assert_eq!(**id, expected_uuid);
            assert_eq!(name, "wave");
            assert!(animated);
        }
        _ => panic!("Expected Emoji mention"),
    }
}

#[test]
fn test_extract_mentions_everyone() {
    let parser = Parser::default();
    let ast = Ast::new(parser.parse("hello @everyone"));

    let mentions: Vec<_> = ast.mentions().collect();
    assert_eq!(mentions.len(), 1);
    match &mentions[0] {
        MentionId::Everyone => {}
        _ => panic!("Expected Everyone mention"),
    }
}

#[test]
fn test_extract_mentions_collect() {
    let parser = Parser::default();
    let user_uuid = uuid!("11111111-1111-1111-1111-111111111111");
    let emoji_uuid = uuid!("22222222-2222-2222-2222-222222222222");
    let ast = Ast::new(parser.parse(
        "hello <@11111111-1111-1111-1111-111111111111> and <:emoji:22222222-2222-2222-2222-222222222222> @everyone"
    ));

    let mentions: MentionIds = ast.mentions().collect();
    assert_eq!(mentions.users.len(), 1);
    assert_eq!(*mentions.users[0], user_uuid);
    assert_eq!(mentions.emojis.len(), 1);
    assert_eq!(*mentions.emojis[0].0, emoji_uuid);
    assert!(mentions.everyone);
}

#[test]
fn test_extract_mentions_role() {
    let parser = Parser::default();
    let expected_uuid = uuid!("12345678-1234-1234-1234-123456789abc");
    let ast = Ast::new(parser.parse("hello <@&12345678-1234-1234-1234-123456789abc>"));

    let mentions: Vec<_> = ast.mentions().collect();
    assert_eq!(mentions.len(), 1);
    match &mentions[0] {
        MentionId::Role(uuid) => assert_eq!(**uuid, expected_uuid),
        _ => panic!("Expected Role mention"),
    }
}

#[test]
fn test_extract_mentions_channel() {
    let parser = Parser::default();
    let expected_uuid = uuid!("12345678-1234-1234-1234-123456789abc");
    let ast = Ast::new(parser.parse("check <#12345678-1234-1234-1234-123456789abc> out"));

    let mentions: Vec<_> = ast.mentions().collect();
    assert_eq!(mentions.len(), 1);
    match &mentions[0] {
        MentionId::Channel(uuid) => assert_eq!(**uuid, expected_uuid),
        _ => panic!("Expected Channel mention"),
    }
}

#[test]
fn test_extract_mentions_all_types() {
    let parser = Parser::default();
    let user_uuid = uuid!("11111111-1111-1111-1111-111111111111");
    let role_uuid = uuid!("22222222-2222-2222-2222-222222222222");
    let channel_uuid = uuid!("33333333-3333-3333-3333-333333333333");
    let emoji_uuid = uuid!("44444444-4444-4444-4444-444444444444");

    let ast = Ast::new(parser.parse(
        "<@11111111-1111-1111-1111-111111111111> <@&22222222-2222-2222-2222-222222222222> <#33333333-3333-3333-3333-333333333333> <:emoji:44444444-4444-4444-4444-444444444444> @everyone"
    ));

    let mentions: MentionIds = ast.mentions().collect();
    assert_eq!(mentions.users.len(), 1);
    assert_eq!(*mentions.users[0], user_uuid);
    assert_eq!(mentions.roles.len(), 1);
    assert_eq!(*mentions.roles[0], role_uuid);
    assert_eq!(mentions.channels.len(), 1);
    assert_eq!(*mentions.channels[0], channel_uuid);
    assert_eq!(mentions.emojis.len(), 1);
    assert_eq!(*mentions.emojis[0].0, emoji_uuid);
    assert!(mentions.everyone);
}
