use std::collections::HashMap;

use lamprey_markdown::query::QueryableExt;
use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use crate::prelude::*;

static DISCORD_USER_MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<@!?([0-9]{17,20})>").unwrap());
static DISCORD_ROLE_MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<@&([0-9]{17,20})>").unwrap());
static DISCORD_CHANNEL_MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<#([0-9]{17,20})>").unwrap());
static DISCORD_EMOJI_MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<a?:([0-9a-zA-Z_]+):([0-9]{17,20})>").unwrap());
static DISCORD_EVERYONE_MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@(everyone|here)").unwrap());

#[derive(Debug)]
pub struct MessageTransformer {
    source: String,
    spans: Vec<MentionSpan>,
}

#[derive(Debug)]
struct MentionSpan {
    start: usize,
    end: usize,
    kind: MentionType,
}

// TODO: use better types than strings?
#[derive(Debug)]
enum MentionType {
    User(String),
    Role(String),
    Channel(String),
    Emoji(MentionEmoji),
    Everyone,
}

#[derive(Debug)]
struct MentionEmoji {
    name: String,
    id: String,
    animated: bool,
}

// TODO: handle role mention permissions: check for role.mentionable or mention everyone permission
// TODO: config option to disable all everyone mentions (maybe configurable per room?)

impl MessageTransformer {
    pub fn parse(message: &bridge_old::MessageData) -> Option<Self> {
        match message {
            bridge_old::MessageData::Lamprey { message, .. } => Self::parse_lamprey(message),
            bridge_old::MessageData::Discord { message } => Self::parse_discord(message),
        }
    }

    fn parse_lamprey(message: &lamprey::Message) -> Option<Self> {
        use lamprey_markdown::ast::inline::MentionData;

        let source = match &message.latest_version.message_type {
            common::v1::types::MessageType::DefaultMarkdown(m)
            | common::v1::types::MessageType::ThreadInitial(m) => m.content.as_ref()?.to_owned(),
            _ => return None,
        };
        let mentions = &message.latest_version.mentions;

        // TODO: if user with an id isn't found (no puppet) or can't be bridged (lamprey mentioning other lamprey users -> discord), default to "@" + resolved_name
        // TODO: also do the above for discord
        // mentions.users[0].resolved_name

        let parser = lamprey_markdown::Parser::new();
        let parsed = parser.parse(&source);
        let spans = parsed
            .tree()
            .iter_mentions()
            .flat_map(|node| {
                let range = node.syntax().text_range();
                let kind = match node.parse() {
                    MentionData::User(id) if mentions.users.iter().any(|i| i.id == id) => {
                        MentionType::User(id.to_string())
                    }
                    MentionData::Role(id) if mentions.roles.iter().any(|i| i.id == id) => {
                        MentionType::Role(id.to_string())
                    }
                    MentionData::Channel(id) if mentions.channels.iter().any(|i| i.id == id) => {
                        MentionType::Channel(id.to_string())
                    }
                    MentionData::Everyone if mentions.everyone => MentionType::Everyone,
                    _ => return None,
                };

                Some(MentionSpan {
                    start: range.start().into(),
                    end: range.end().into(),
                    kind,
                })
            })
            .collect();

        Some(Self { source, spans })
    }

    fn parse_discord(message: &discord::Message) -> Option<Self> {
        let source = message.content.clone();
        let mut spans = Vec::new();

        // TODO: skip parsing in code blocks

        for cap in DISCORD_USER_MENTION_RE.captures_iter(&source) {
            let range = cap.get(0).unwrap().range();
            spans.push(MentionSpan {
                start: range.start,
                end: range.end,
                kind: MentionType::User(cap[1].to_string()),
            });
        }

        for cap in DISCORD_ROLE_MENTION_RE.captures_iter(&source) {
            let range = cap.get(0).unwrap().range();
            spans.push(MentionSpan {
                start: range.start,
                end: range.end,
                kind: MentionType::Role(cap[1].to_string()),
            });
        }

        for cap in DISCORD_CHANNEL_MENTION_RE.captures_iter(&source) {
            let range = cap.get(0).unwrap().range();
            spans.push(MentionSpan {
                start: range.start,
                end: range.end,
                kind: MentionType::Channel(cap[1].to_string()),
            });
        }

        for cap in DISCORD_EMOJI_MENTION_RE.captures_iter(&source) {
            let range = cap.get(0).unwrap().range();
            let animated = cap[0].starts_with("<a:");
            spans.push(MentionSpan {
                start: range.start,
                end: range.end,
                kind: MentionType::Emoji(MentionEmoji {
                    name: cap[1].to_string(),
                    id: cap[2].to_string(),
                    animated,
                }),
            });
        }

        // TODO: use DISCORD_EVERYONE_MENTION_RE

        spans.sort_by_key(|s| s.start);

        Some(Self { source, spans })
    }

    pub fn mentioned_users(&self) -> std::collections::HashSet<&str> {
        self.spans
            .iter()
            .filter_map(|s| match &s.kind {
                MentionType::User(id) => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn mentioned_roles(&self) -> std::collections::HashSet<&str> {
        self.spans
            .iter()
            .filter_map(|s| match &s.kind {
                MentionType::Role(id) => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn mentioned_channels(&self) -> std::collections::HashSet<&str> {
        self.spans
            .iter()
            .filter_map(|s| match &s.kind {
                MentionType::Channel(id) => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn has_everyone(&self) -> bool {
        self.spans
            .iter()
            .any(|s| matches!(s.kind, MentionType::Everyone))
    }

    // TODO: handle custom emoji
    pub fn to_discord(
        &self,
        user_mappings: &HashMap<String, discord::UserId>,
        role_mappings: &HashMap<String, discord::RoleId>,
        channel_mappings: &HashMap<String, discord::ChannelId>,
    ) -> (String, discord::CreateAllowedMentions) {
        let mut result = String::new();
        let mut last_end = 0;

        for span in &self.spans {
            result.push_str(&self.source[last_end..span.start]);

            match &span.kind {
                MentionType::User(id) => {
                    if let Some(dc_id) = user_mappings.get(id) {
                        result.push_str(&format!("<@{}>", dc_id));
                    } else {
                        result.push_str(&self.source[span.start..span.end]);
                    }
                }
                MentionType::Role(id) => {
                    if let Some(dc_id) = role_mappings.get(id) {
                        result.push_str(&format!("<@&{}>", dc_id));
                    } else {
                        result.push_str(&self.source[span.start..span.end]);
                    }
                }
                MentionType::Channel(id) => {
                    if let Some(dc_id) = channel_mappings.get(id) {
                        result.push_str(&format!("<#{}>", dc_id));
                    } else {
                        result.push_str(&self.source[span.start..span.end]);
                    }
                }
                MentionType::Emoji(_emoji) => {
                    result.push_str(&self.source[span.start..span.end]);
                }
                MentionType::Everyone => {
                    result.push_str("@everyone");
                }
            }

            last_end = span.end;
        }
        result.push_str(&self.source[last_end..]);

        let allowed_mentions = discord::CreateAllowedMentions::new()
            .everyone(false)
            // .roles([])
            .all_users(true);

        (result, allowed_mentions)
    }

    // TODO: handle custom emoji
    pub fn to_lamprey(
        &self,
        user_mappings: &HashMap<String, lamprey::UserId>,
        role_mappings: &HashMap<String, lamprey::RoleId>,
        channel_mappings: &HashMap<String, lamprey::ChannelId>,
    ) -> (String, lamprey::ParseMentions) {
        let mut result = String::new();
        let mut last_end = 0;

        for span in &self.spans {
            result.push_str(&self.source[last_end..span.start]);

            match &span.kind {
                MentionType::User(id) => {
                    if let Some(lamprey_id) = user_mappings.get(id) {
                        result.push_str(&format!("<@{}>", Uuid::from(*lamprey_id)));
                    } else {
                        result.push_str(&self.source[span.start..span.end]);
                    }
                }
                MentionType::Role(id) => {
                    if let Some(lamprey_id) = role_mappings.get(id) {
                        result.push_str(&format!("<@&{}>", Uuid::from(*lamprey_id)));
                    } else {
                        result.push_str(&self.source[span.start..span.end]);
                    }
                }
                MentionType::Channel(id) => {
                    if let Some(lamprey_id) = channel_mappings.get(id) {
                        result.push_str(&format!("<#{}>", Uuid::from(*lamprey_id)));
                    } else {
                        result.push_str(&self.source[span.start..span.end]);
                    }
                }
                MentionType::Emoji(_emoji) => {
                    result.push_str(&self.source[span.start..span.end]);
                }
                MentionType::Everyone => {
                    result.push_str("@everyone");
                }
            }

            last_end = span.end;
        }
        result.push_str(&self.source[last_end..]);

        let mentions = lamprey::ParseMentions {
            users: None,
            roles: Some(vec![]),
            everyone: false,
        };

        (result, mentions)
    }
}

#[cfg(test)]
mod tests {
    // TODO: copy from old code
}
