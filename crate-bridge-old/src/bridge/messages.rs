use serenity::all::{ChannelId as DcChannelId, GuildId as DcGuildId};

#[derive(Debug, Clone)]
pub enum BridgeMessage {
    LampreyThreadCreate {
        thread: common::v1::types::Channel,
        discord_guild_id: DcGuildId,
    },
    DiscordChannelCreate {
        guild_id: DcGuildId,
        channel_id: DcChannelId,
        channel_name: String,
        channel_type: serenity::all::ChannelType,
        parent_id: Option<DcChannelId>,
    },
}
