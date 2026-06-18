use common::v2::types::ChannelId;
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption, GuildId,
    InteractionContext, Permissions,
};

use crate::prelude::*;

/// a slash command from discord
pub enum SlashCommand {
    /// check if the bridge is alive
    Ping,

    LinkGuild {
        discord_guild_id: GuildId,
        lamprey_room_id: lamprey::RoomId,
        backfill: bool,
        continuous: bool,
    },

    LinkChannel {
        discord_channel_id: ChannelId,
        lamprey_channel_id: lamprey::ChannelId,
        backfill: bool,
    },

    UnlinkGuild {
        discord_guild_id: GuildId,
    },

    UnlinkChannel {
        discord_channel_id: ChannelId,
    },
}

/// get discord slash commands
fn get_commands() -> Vec<CreateCommand> {
    let ping = CreateCommand::new("ping")
        .description("check if the bridge is alive")
        .default_member_permissions(Permissions::from_bits_truncate(536870944));

    let link = CreateCommand::new("link")
            .description("link something to lamprey")
            .default_member_permissions(Permissions::from_bits_truncate(536870944))
            .contexts(vec![InteractionContext::Guild])
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "guild", "link this guild (server)")
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "room_id",
                            "the uuid of the room to link to",
                        )
                        .required(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "backfill",
                            "whether to clone the full history of every channel",
                        )
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "continuous",
                            "whether to create new portals as channels and threads are created (this is bidirectional)",
                        )
                    ),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "channel", "link this channel")
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "channel_id",
                            "the uuid of the channel to link to",
                        )
                        .required(true),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::Boolean,
                            "backfill",
                            "whether to clone the full history of this channel",
                        )
                    )
            );

    let unlink = CreateCommand::new("unlink")
        .description("unlink something from lamprey")
        .default_member_permissions(Permissions::from_bits_truncate(536870944))
        .contexts(vec![InteractionContext::Guild])
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "guild",
            "unlink this guild (server)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "channel",
            "unlink this channel",
        ));

    // TODO: command(s) to edit an existing realm/portal
    // TODO: command(s) to moderate (kick, ban, timeout) users on other platforms

    vec![ping, link, unlink]
}

fn parse_interaction(interaction: &CommandInteraction) -> Result<SlashCommand> {
    match interaction.data.name.as_str() {
        "ping" => todo!("parse"),
        "link" => todo!("parse"),
        "unlink" => todo!("parse"),
        _ => todo!("return error"),
    }
}
