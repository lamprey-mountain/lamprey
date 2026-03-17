//! Discord → Lamprey mention conversion
//!
//! Parses Discord mention formats and converts them to Lamprey UUID mentions

use anyhow::Result;
use common::v1::types::{ParseMentions, UserId};
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::all::GuildId as DcGuildId;

use crate::bridge_common::Globals;
use crate::db::Data;

// Discord mention regex patterns
// User mentions: <@USER_ID> or <@!USER_ID> (deprecated nickname format)
static USER_MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<@!?([0-9]{17,20})>").unwrap());

// Role mentions: <@&ROLE_ID>
static ROLE_MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<@&([0-9]{17,20})>").unwrap());

// Channel mentions: <#CHANNEL_ID>
static CHANNEL_MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<#([0-9]{17,20})>").unwrap());

// Emoji mentions: <:name:id> or <a:name:id> (animated)
static EMOJI_MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<a?:([0-9a-zA-Z_]+):([0-9]{17,20})>").unwrap());

// @everyone and @here
static EVERYONE_MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@(everyone|here)").unwrap());

// Code blocks to skip mention parsing
static CODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)```[\s\S]*?```|`[^`\n]+`").unwrap());

/// Result of parsing Discord mentions
#[derive(Debug, Default)]
pub struct DiscordMentionsParsed {
    /// User IDs that were mentioned (Discord snowflakes)
    pub users: Vec<String>,
    /// Role IDs that were mentioned (Discord snowflakes)
    pub roles: Vec<String>,
    /// Channel IDs that were mentioned (Discord snowflakes)
    pub channels: Vec<String>,
    /// Whether @everyone or @here was mentioned
    pub everyone: bool,
    /// Sanitized content with Discord mentions replaced by placeholder text
    pub content_sanitized: String,
}

/// Parse Discord mentions from message content
///
/// This extracts all mention IDs from the content but does NOT resolve them to Lamprey IDs.
/// The content is sanitized by removing code blocks to avoid parsing mentions inside code.
pub fn parse_discord_mentions(content: &str) -> DiscordMentionsParsed {
    let sanitized_owned;
    let content = if content.contains('`') {
        sanitized_owned =
            CODE_RE.replace_all(content, |caps: &regex::Captures| " ".repeat(caps[0].len()));
        &*sanitized_owned
    } else {
        content
    };

    let users = USER_MENTION_RE
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect::<Vec<_>>();

    let roles = ROLE_MENTION_RE
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect::<Vec<_>>();

    let channels = CHANNEL_MENTION_RE
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect::<Vec<_>>();

    let everyone = EVERYONE_MENTION_RE.is_match(content);

    // Create sanitized content by replacing mentions with readable text
    let content_sanitized = USER_MENTION_RE.replace_all(content, "@user").to_string();
    let content_sanitized = ROLE_MENTION_RE
        .replace_all(&content_sanitized, "@role")
        .to_string();
    let content_sanitized = CHANNEL_MENTION_RE
        .replace_all(&content_sanitized, "#channel")
        .to_string();
    let content_sanitized = EMOJI_MENTION_RE
        .replace_all(&content_sanitized, ":emoji:")
        .to_string();

    DiscordMentionsParsed {
        users,
        roles,
        channels,
        everyone,
        content_sanitized,
    }
}

/// Resolve Discord mention IDs to Lamprey UUIDs using the puppet system
///
/// This queries the database to find matching puppets for each Discord user/role mention.
pub async fn resolve_discord_mentions(
    globals: &Globals,
    parsed: &DiscordMentionsParsed,
    _discord_guild_id: DcGuildId,
) -> Result<ParseMentions> {
    let mut users = Vec::new();

    // Resolve user mentions
    for dc_user_id in &parsed.users {
        if let Some(puppet) = Data::get_puppet(globals, "discord", dc_user_id).await? {
            users.push(UserId::from(puppet.id));
        }
        // If no puppet found, the mention is silently dropped (user not bridged)
    }

    // Resolve role mentions
    // Note: This requires a reverse lookup - we need to find Lamprey role by Discord role ID
    // For now, we'll need to add a query method for this
    // TODO: Add get_lamprey_role_by_discord_id query method

    Ok(ParseMentions {
        users: if users.is_empty() { None } else { Some(users) },
        roles: None, // Role mentions not yet supported
        everyone: parsed.everyone,
    })
}

/// Convert Discord mentions to Lamprey UUID mention format in content
///
/// This replaces Discord snowflake mentions with Lamprey UUID mentions.
pub async fn convert_discord_mentions_to_lamprey(
    globals: &Globals,
    content: &str,
    discord_guild_id: DcGuildId,
) -> Result<(String, ParseMentions)> {
    let parsed = parse_discord_mentions(content);
    let mentions = resolve_discord_mentions(globals, &parsed, discord_guild_id).await?;

    // Replace Discord mentions with Lamprey UUID mentions
    let mut result = content.to_string();

    // Replace user mentions
    for (dc_id, lamprey_id) in parsed
        .users
        .iter()
        .zip(mentions.users.as_ref().into_iter().flatten())
    {
        // Replace both <@id> and <@!id> formats
        result = result
            .replace(&format!("<@{}>", dc_id), &format!("<@{}>", lamprey_id))
            .replace(&format!("<@!{}>", dc_id), &format!("<@{}>", lamprey_id));
    }

    // For unresolved mentions, keep the original Discord format or strip
    // (currently we just leave them as-is since they won't be rendered anyway)

    Ok((result, mentions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_mentions() {
        let content = "Hello <@123456789012345678> and <@!987654321098765432>!";
        let parsed = parse_discord_mentions(content);
        assert_eq!(parsed.users.len(), 2);
        assert_eq!(parsed.users[0], "123456789012345678");
        assert_eq!(parsed.users[1], "987654321098765432");
    }

    #[test]
    fn test_parse_role_mentions() {
        let content = "Check out <@&111111111111111111> role!";
        let parsed = parse_discord_mentions(content);
        assert_eq!(parsed.roles.len(), 1);
        assert_eq!(parsed.roles[0], "111111111111111111");
    }

    #[test]
    fn test_parse_everyone() {
        let content = "@everyone wake up!";
        let parsed = parse_discord_mentions(content);
        assert!(parsed.everyone);
    }

    #[test]
    fn test_parse_here() {
        let content = "@here anyone here?";
        let parsed = parse_discord_mentions(content);
        assert!(parsed.everyone);
    }

    #[test]
    fn test_skip_code_blocks() {
        let content = "Don't parse `<@123456789012345678>` in code";
        let parsed = parse_discord_mentions(content);
        assert_eq!(parsed.users.len(), 0);
    }

    #[test]
    fn test_skip_multiline_code_blocks() {
        let content = "```\n<@123456789012345678>\n```";
        let parsed = parse_discord_mentions(content);
        assert_eq!(parsed.users.len(), 0);
    }
}
