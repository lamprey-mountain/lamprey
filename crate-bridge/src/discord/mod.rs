use crate::{bridge::BridgeHandle, config::DiscordConfig};

mod interactions;

// re export discord (serenity) types
pub use serenity::all::{ChannelId, GuildId, MessageId, UserId, AttachmentId};

pub fn spawn(bridge: BridgeHandle, config: DiscordConfig) {
    todo!()
}
