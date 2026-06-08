use common::v1::types::{EmojiId, ParseMentions};
use lamprey_markdown::ast::MentionIds as AstMentionIds;
use lamprey_markdown::parser::{ParseOptions, Parser, SyntaxNode};
use lamprey_markdown::renderer::{MarkdownRenderer, Renderer};
use lamprey_markdown::transformer::{Pipeline, StripEmoji};
use lamprey_markdown::Ast;

use crate::types::MentionsIds;

// TODO: rename to parse_mention_ids
pub fn parse(content: &str, options: &ParseMentions) -> MentionsIds {
    let parser = Parser::default();
    let parsed = parser.parse(content);
    let ast = Ast::new(parsed);

    let mention_ids: AstMentionIds = ast.mentions().collect();

    let users = if let Some(allowed_users) = &options.users {
        mention_ids
            .users
            .into_iter()
            .filter(|id| allowed_users.contains(id))
            .collect()
    } else {
        mention_ids.users
    };

    let roles = if let Some(allowed_roles) = &options.roles {
        mention_ids
            .roles
            .into_iter()
            .filter(|id| allowed_roles.contains(id))
            .collect()
    } else {
        mention_ids.roles
    };

    let channels = mention_ids.channels;
    let emojis = mention_ids
        .emojis
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();

    let everyone = options.everyone && mention_ids.everyone;

    MentionsIds {
        users,
        roles,
        channels,
        emojis,
        everyone,
    }
}

pub fn strip_emoji(content: &str, allowed_emoji: &[EmojiId]) -> String {
    let parser = Parser::new(ParseOptions::default());
    let ast = Ast::new(parser.parse(content));

    let mut pipeline = Pipeline::new();
    pipeline.add_transform(StripEmoji::from_emoji_ids(allowed_emoji.to_vec()));
    let transformed = pipeline.apply(&ast.syntax());
    let transformed_node = SyntaxNode::new_root(transformed);
    MarkdownRenderer.render(&transformed_node)
}

pub fn strip_emoji2(content: &str, allowed_emoji: &[EmojiId]) -> String {
    let parser = Parser::new();
    let parsed = parser.parse(content);
    // parsed.iter_links();
    // let ast = Ast::new(parser.parse(content));

    // let mut pipeline = Pipeline::new();
    // pipeline.add_transform(StripEmoji::from_emoji_ids(allowed_emoji.to_vec()));
    // let transformed = pipeline.apply(&ast.syntax());
    // let transformed_node = SyntaxNode::new_root(transformed);
    // MarkdownRenderer.render(&transformed_node)
    todo!()
}

// TODO: add parse_links (copy from crate-backend/src/services/messages/links.rs?)
