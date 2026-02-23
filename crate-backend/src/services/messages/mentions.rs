use common::v1::types::{EmojiId, ParseMentions, RoleId, UserId};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use uuid::Uuid;

use crate::types::MentionsIds;

static USER_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>")
        .unwrap()
});
static ROLE_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>")
        .unwrap()
});
static CHANNEL_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<#([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>")
        .unwrap()
});
static EMOJI_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"<a?:(\w+):([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{12})>",
    )
    .unwrap()
});
static EVERYONE_MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@everyone").unwrap());
static CODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)```[\s\S]*?```|`[^`\n]+`").unwrap());

pub fn parse(content: &str, options: &ParseMentions) -> MentionsIds {
    let sanitized_owned;
    let content = if content.contains('`') {
        sanitized_owned =
            CODE_RE.replace_all(content, |caps: &regex::Captures| " ".repeat(caps[0].len()));
        &*sanitized_owned
    } else {
        content
    };

    let users = options
        .users
        .as_ref()
        .map(|allowed_users| {
            USER_MENTION_RE
                .captures_iter(content)
                .filter_map(|cap| {
                    let id = Uuid::parse_str(&cap[1]).ok()?.into();
                    if allowed_users.contains(&id) {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<UserId>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_else(|| {
            USER_MENTION_RE
                .captures_iter(content)
                .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
                .collect::<HashSet<UserId>>()
                .into_iter()
                .collect()
        });

    let roles = options
        .roles
        .as_ref()
        .map(|allowed_roles| {
            ROLE_MENTION_RE
                .captures_iter(content)
                .filter_map(|cap| {
                    let id = Uuid::parse_str(&cap[1]).ok()?.into();
                    if allowed_roles.contains(&id) {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<RoleId>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_else(|| {
            ROLE_MENTION_RE
                .captures_iter(content)
                .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
                .collect::<HashSet<RoleId>>()
                .into_iter()
                .collect()
        });

    let channels = CHANNEL_MENTION_RE
        .captures_iter(content)
        .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let emojis = EMOJI_MENTION_RE
        .captures_iter(content)
        .filter_map(|cap| Uuid::parse_str(&cap[2]).ok().map(Into::into))
        .collect::<HashSet<EmojiId>>()
        .into_iter()
        .collect();

    let everyone = options.everyone && EVERYONE_MENTION_RE.is_match(content);

    MentionsIds {
        users,
        roles,
        channels,
        emojis,
        everyone,
    }
}

pub fn strip_emoji(content: &str, allowed_emoji: &[EmojiId]) -> String {
    static STRIP_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?s)(```[\s\S]*?```|`[^`\n]+`)|(<a?:(\w+):([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{12})>)").unwrap()
    });

    STRIP_RE
        .replace_all(content, |caps: &regex::Captures| {
            if let Some(code) = caps.get(1) {
                return code.as_str().to_string();
            }

            let emoji_id = Uuid::parse_str(&caps[4]).ok().map(EmojiId::from);
            match emoji_id {
                Some(id) if allowed_emoji.contains(&id) => caps
                    .get(0)
                    .expect("index 0 is always Some")
                    .as_str()
                    .to_string(),
                _ => {
                    let name = caps.get(3).expect("this should always exist").as_str();
                    format!(":{}:", name)
                }
            }
        })
        .to_string()
}
