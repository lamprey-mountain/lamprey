//! Mention conversion between Lamprey and Discord formats
//!
//! This module handles bidirectional conversion of mentions:
//! - Discord -> Lamprey: Parse Discord snowflake mentions and convert to UUID mentions
//! - Lamprey -> Discord: Convert UUID mentions to Discord snowflake mentions

mod discord_to_lamprey;
mod lamprey_to_discord;

pub use discord_to_lamprey::*;
pub use lamprey_to_discord::*;
