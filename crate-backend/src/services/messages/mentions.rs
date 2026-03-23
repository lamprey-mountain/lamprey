use common::v1::types::{EmojiId, ParseMentions};
use lamprey_markdown::ast::MentionIds as AstMentionIds;
use lamprey_markdown::render::StripEmojiReader;
use lamprey_markdown::{Ast, Parser};

use crate::types::MentionsIds;

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
    let parser = Parser::default();
    let parsed = parser.parse(content);
    let ast = Ast::new(parsed);
    let reader = StripEmojiReader::new(allowed_emoji.to_vec());
    reader.read(&ast)
}
