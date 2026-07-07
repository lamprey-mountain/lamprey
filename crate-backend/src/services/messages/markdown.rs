use crate::types::MentionsIds;
use common::v1::types::{EmojiId, ParseMentions};
use lamprey_markdown::{
    Parser,
    ast::inline::{Emoji, MentionData},
    query::QueryableExt,
    transform::StripEmoji,
};
use url::Url;

pub fn parse(content: &str, options: &ParseMentions) -> MentionsIds {
    let parser = Parser::new();
    let parsed = parser.parse(content);

    let mut users = Vec::new();
    let mut roles = Vec::new();
    let mut channels = Vec::new();
    let mut emojis = Vec::new();
    let mut everyone = false;

    for mention in parsed.tree().iter_mentions() {
        match mention.parse() {
            MentionData::User(u) => users.push(u),
            MentionData::Role(r) => roles.push(r),
            MentionData::Channel(c) => channels.push(c),
            MentionData::Everyone => everyone = true,
        }
    }

    for emoji in parsed.tree().iter_emoji() {
        if let Emoji::Custom(e) = emoji {
            emojis.push(e.parse().id.into());
        }
    }

    let users = if let Some(allowed_users) = &options.users {
        users
            .into_iter()
            .filter(|id| allowed_users.contains(id))
            .collect()
    } else {
        users
    };

    let roles = if let Some(allowed_roles) = &options.roles {
        roles
            .into_iter()
            .filter(|id| allowed_roles.contains(id))
            .collect()
    } else {
        roles
    };

    everyone = options.everyone && everyone;

    MentionsIds {
        users,
        roles,
        channels,
        emojis,
        everyone,
    }
}

pub fn strip_emoji(content: &str, allowed_emoji: &[EmojiId]) -> String {
    let parser = Parser::new();
    let parsed = parser.parse(content);
    let transformer = StripEmoji {
        allowed: allowed_emoji.iter().map(|id| **id).collect(),
    };

    let transformed = parsed.transform(&transformer);
    transformed.to_markdown()
}

pub fn extract_links(content: &str) -> Vec<Url> {
    let parser = Parser::new();
    let parsed = parser.parse(content);
    parsed
        .tree()
        .iter_links()
        .filter_map(|link| Url::parse(&link.href()).ok())
        .collect()
}
