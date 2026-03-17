//! Lamprey → Discord mention conversion
//!
//! Converts Lamprey UUID mentions to Discord snowflake mentions

use anyhow::Result;
use common::v1::types::{Mentions, RoleId, UserId};
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::all::CreateAllowedMentions;
use std::collections::HashMap;
use uuid::Uuid;

use crate::bridge_common::Globals;
use crate::db::Data;

// Lamprey mention regex patterns (UUID format)
static USER_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>")
        .unwrap()
});

static ROLE_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>")
        .unwrap()
});

// Code blocks to skip mention parsing
static CODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)```[\s\S]*?```|`[^`\n]+`").unwrap());

/// Resolved Discord mention data
#[derive(Debug)]
pub struct ResolvedDiscordMention {
    /// Discord snowflake ID for user
    pub user_id: Option<String>,
    /// Discord snowflake ID for role
    pub role_id: Option<String>,
    /// Whether this mention should ping (for allowed_mentions config)
    pub should_ping: bool,
}

/// Result of converting Lamprey mentions to Discord format
#[derive(Debug)]
pub struct LampreyMentionsConverted {
    /// Content with Discord mention format
    pub content: String,
    /// Discord AllowedMentions configuration
    pub allowed_mentions: CreateAllowedMentions,
    /// Resolved user mentions (Discord snowflakes)
    pub user_ids: Vec<String>,
    /// Resolved role mentions (Discord snowflakes)
    pub role_ids: Vec<String>,
    /// Whether @everyone/@here was mentioned
    pub everyone: bool,
}

/// Resolve Lamprey user IDs to Discord snowflakes using the puppet system
async fn resolve_user_ids(
    globals: &Globals,
    user_ids: &[UserId],
) -> Result<HashMap<UserId, String>> {
    let mut resolved = HashMap::new();

    for &lamprey_user_id in user_ids {
        let lamprey_uuid = Uuid::from(lamprey_user_id);

        // Query: SELECT ext_id FROM puppet WHERE id = ? AND ext_platform = 'discord'
        if let Some(dc_id) =
            Data::get_puppet_by_lamprey_id(globals, "discord", lamprey_uuid).await?
        {
            resolved.insert(lamprey_user_id, dc_id);
        }
    }

    Ok(resolved)
}

/// Resolve Lamprey role IDs to Discord role snowflakes
///
/// This will use the discord_role_mapping table (to be created)
async fn resolve_role_ids(
    globals: &Globals,
    role_ids: &[RoleId],
    discord_guild_id: serenity::all::GuildId,
) -> Result<HashMap<RoleId, String>> {
    let mut resolved = HashMap::new();

    for &lamprey_role_id in role_ids {
        // RoleId is a wrapper around Uuid, dereference it
        let lamprey_uuid = Uuid::from(lamprey_role_id);
        if let Some(dc_role_id) =
            Data::get_discord_role_mapping(globals, lamprey_uuid, discord_guild_id).await?
        {
            resolved.insert(lamprey_role_id, dc_role_id);
        }
    }

    Ok(resolved)
}

/// Parse Lamprey mentions from content (similar to backend mentions.rs)
fn parse_lamprey_mentions(content: &str) -> (Vec<UserId>, Vec<RoleId>, bool) {
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
        .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(UserId::from))
        .collect::<Vec<_>>();

    let roles = ROLE_MENTION_RE
        .captures_iter(content)
        .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(RoleId::from))
        .collect::<Vec<_>>();

    // Lamprey doesn't have @everyone in the same way, but we can check for it
    let everyone = content.contains("@everyone") || content.contains("@here");

    (users, roles, everyone)
}

/// Convert Lamprey mentions to Discord format
///
/// This replaces UUID mentions with Discord snowflake mentions and creates
/// the appropriate AllowedMentions configuration.
pub async fn convert_lamprey_to_discord(
    globals: &Globals,
    content: &str,
    mentions: &Mentions,
    discord_guild_id: serenity::all::GuildId,
) -> Result<LampreyMentionsConverted> {
    // Parse mentions from content
    let (parsed_users, parsed_roles, parsed_everyone) = parse_lamprey_mentions(content);

    // Resolve to Discord IDs
    let resolved_users = resolve_user_ids(globals, &parsed_users).await?;
    let resolved_roles = resolve_role_ids(globals, &parsed_roles, discord_guild_id).await?;

    // Build allowed mentions list
    let mut user_ids = Vec::new();
    let mut role_ids = Vec::new();

    // Only include mentions that are both in content AND in the mentions struct
    // (the mentions struct is what the backend parsed and validated)
    for user_mention in &mentions.users {
        if let Some(dc_id) = resolved_users.get(&user_mention.id) {
            user_ids.push(dc_id.clone());
        }
    }

    for role_mention in &mentions.roles {
        if let Some(dc_id) = resolved_roles.get(&role_mention.id) {
            role_ids.push(dc_id.clone());
        }
    }

    // Replace mentions in content
    let mut result = content.to_string();

    // Replace user mentions
    for (&lamprey_id, dc_id) in &resolved_users {
        let uuid_str = Uuid::from(lamprey_id).to_string();
        result = result.replace(&format!("<@{}>", uuid_str), &format!("<@{}>", dc_id));
    }

    // Replace role mentions
    for (&lamprey_id, dc_id) in &resolved_roles {
        let uuid_str = Uuid::from(lamprey_id).to_string();
        result = result.replace(&format!("<@&{}>", uuid_str), &format!("<@&{}>", dc_id));
    }

    // Handle @everyone/@here
    let everyone = parsed_everyone && mentions.everyone;

    // Build CreateAllowedMentions
    let allowed_mentions = CreateAllowedMentions::new()
        .everyone(everyone)
        .all_roles(false) // Don't allow all roles, we specify explicitly
        .all_users(false) // Don't allow all users, we specify explicitly
        .roles(
            role_ids
                .iter()
                .filter_map(|id| id.parse::<u64>().ok())
                .map(serenity::all::RoleId::from),
        )
        .users(
            user_ids
                .iter()
                .filter_map(|id| id.parse::<u64>().ok())
                .map(serenity::all::UserId::from),
        );

    Ok(LampreyMentionsConverted {
        content: result,
        allowed_mentions,
        user_ids,
        role_ids,
        everyone,
    })
}

/// Simple conversion for when you just need to replace mentions without validation
///
/// This is useful for editing messages where mentions are already resolved
pub async fn replace_lamprey_mentions_simple(
    globals: &Globals,
    content: &str,
    discord_guild_id: serenity::all::GuildId,
) -> Result<String> {
    let (users, roles, _) = parse_lamprey_mentions(content);

    let resolved_users = resolve_user_ids(globals, &users).await?;
    let resolved_roles = resolve_role_ids(globals, &roles, discord_guild_id).await?;

    let mut result = content.to_string();

    for (&lamprey_id, dc_id) in &resolved_users {
        let uuid_str = Uuid::from(lamprey_id).to_string();
        result = result.replace(&format!("<@{}>", uuid_str), &format!("<@{}>", dc_id));
    }

    for (&lamprey_id, dc_id) in &resolved_roles {
        let uuid_str = Uuid::from(lamprey_id).to_string();
        result = result.replace(&format!("<@&{}>", uuid_str), &format!("<@&{}>", dc_id));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lamprey_user_mentions() {
        let content = "Hello <@550e8400-e29b-41d4-a716-446655440000>!";
        let (users, roles, everyone) = parse_lamprey_mentions(content);
        assert_eq!(users.len(), 1);
        assert_eq!(roles.len(), 0);
        assert!(!everyone);
    }

    #[test]
    fn test_parse_lamprey_role_mentions() {
        let content = "Check <@&550e8400-e29b-41d4-a716-446655440001>!";
        let (users, roles, everyone) = parse_lamprey_mentions(content);
        assert_eq!(users.len(), 0);
        assert_eq!(roles.len(), 1);
        assert!(!everyone);
    }

    #[test]
    fn test_skip_code_blocks() {
        let content = "Don't parse `<@550e8400-e29b-41d4-a716-446655440000>` in code";
        let (users, roles, everyone) = parse_lamprey_mentions(content);
        assert_eq!(users.len(), 0);
        assert_eq!(roles.len(), 0);
        assert!(!everyone);
    }
}
