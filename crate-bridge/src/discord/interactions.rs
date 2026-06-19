use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, InteractionContext, Permissions,
};

use crate::prelude::*;

/// a slash command from discord
#[derive(Debug)]
pub enum SlashCommand {
    /// check if the bridge is alive
    Ping,

    LinkGuild {
        discord_guild_id: discord::GuildId,
        lamprey_room_id: lamprey::RoomId,
        backfill: bool,
        continuous: bool,
    },

    LinkChannel {
        discord_channel_id: discord::ChannelId,
        lamprey_channel_id: lamprey::ChannelId,
        backfill: bool,
    },

    UnlinkGuild {
        discord_guild_id: discord::GuildId,
    },

    UnlinkChannel {
        discord_channel_id: discord::ChannelId,
    },
}

/// get discord slash commands
pub fn get_commands() -> Vec<CreateCommand> {
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

pub fn parse_interaction(interaction: &CommandInteraction) -> Result<SlashCommand> {
    match interaction.data.name.as_str() {
        "ping" => Ok(SlashCommand::Ping),
        "link" => {
            let subcommand = interaction
                .data
                .options
                .get(0)
                .ok_or_else(|| anyhow::anyhow!("missing subcommand"))?;
            let CommandDataOptionValue::SubCommand(options) = &subcommand.value else {
                return Err(anyhow::anyhow!("invalid subcommand"));
            };

            match subcommand.name.as_str() {
                "guild" => {
                    let mut room_id_str = None;
                    let mut backfill = false;
                    let mut continuous = false;
                    for opt in options {
                        match opt.name.as_str() {
                            "room_id" => room_id_str = opt.value.as_str().map(|s| s.to_owned()),
                            "backfill" => backfill = opt.value.as_bool().unwrap_or(false),
                            "continuous" => continuous = opt.value.as_bool().unwrap_or(false),
                            _ => {}
                        }
                    }
                    let room_id = room_id_str
                        .ok_or_else(|| anyhow::anyhow!("missing room_id"))?
                        .parse()?;
                    Ok(SlashCommand::LinkGuild {
                        discord_guild_id: interaction
                            .guild_id
                            .ok_or_else(|| anyhow::anyhow!("not in guild"))?,
                        lamprey_room_id: room_id,
                        backfill,
                        continuous,
                    })
                }
                "channel" => {
                    let mut channel_id_str = None;
                    let mut backfill = false;
                    for opt in options {
                        match opt.name.as_str() {
                            "channel_id" => {
                                channel_id_str = opt.value.as_str().map(|s| s.to_owned())
                            }
                            "backfill" => backfill = opt.value.as_bool().unwrap_or(false),
                            _ => {}
                        }
                    }
                    let channel_id = channel_id_str
                        .ok_or_else(|| anyhow::anyhow!("missing channel_id"))?
                        .parse()?;
                    Ok(SlashCommand::LinkChannel {
                        discord_channel_id: interaction.channel_id.into(),
                        lamprey_channel_id: channel_id,
                        backfill,
                    })
                }
                _ => Err(anyhow::anyhow!("unknown link subcommand")),
            }
        }
        "unlink" => {
            let subcommand = interaction
                .data
                .options
                .get(0)
                .ok_or_else(|| anyhow::anyhow!("missing subcommand"))?;
            match subcommand.name.as_str() {
                "guild" => Ok(SlashCommand::UnlinkGuild {
                    discord_guild_id: interaction
                        .guild_id
                        .ok_or_else(|| anyhow::anyhow!("not in guild"))?,
                }),
                "channel" => Ok(SlashCommand::UnlinkChannel {
                    discord_channel_id: interaction.channel_id.into(),
                }),
                _ => Err(anyhow::anyhow!("unknown unlink subcommand")),
            }
        }
        _ => Err(anyhow::anyhow!("unknown command")),
    }
}
