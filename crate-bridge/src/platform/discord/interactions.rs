use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, GuildId, InteractionContext, Permissions,
};

use crate::{config::Config, prelude::*};

/// a slash command from discord
#[derive(Debug)]
pub struct SlashCommand {
    pub interaction: CommandInteraction,
    pub inner: SlashCommandType,
}

impl SlashCommand {
    pub fn channel_id(&self) -> ChannelId {
        self.interaction.channel_id
    }

    pub fn guild_id(&self) -> GuildId {
        self.interaction
            .guild_id
            .expect("bridge slash commands are only allowed in guilds")
    }
}

/// a slash command from discord
#[derive(Debug)]
pub enum SlashCommandType {
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
pub fn get_commands(config: &Config) -> Vec<CreateCommand> {
    let mut ping = CreateCommand::new("ping").description("check if the bridge is alive");

    if !config.disable_discord_slash_command_permission_checks {
        ping = ping.default_member_permissions(Permissions::from_bits_truncate(536870944));
    }

    let mut link = CreateCommand::new("link")
        .description("link something to lamprey")
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

    if !config.disable_discord_slash_command_permission_checks {
        link = link.default_member_permissions(Permissions::from_bits_truncate(536870944));
    }

    let mut unlink = CreateCommand::new("unlink")
        .description("unlink something from lamprey")
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

    if !config.disable_discord_slash_command_permission_checks {
        unlink = unlink.default_member_permissions(Permissions::from_bits_truncate(536870944));
    }

    // TODO: command(s) to edit an existing realm/portal
    // TODO: command(s) to moderate (kick, ban, timeout) users on other platforms

    vec![ping, link, unlink]
}

pub fn parse_interaction(interaction: CommandInteraction) -> Result<SlashCommand> {
    let inner = match interaction.data.name.as_str() {
        "ping" => SlashCommandType::Ping,
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
                    SlashCommandType::LinkGuild {
                        discord_guild_id: interaction
                            .guild_id
                            .ok_or_else(|| anyhow::anyhow!("not in guild"))?,
                        lamprey_room_id: room_id,
                        backfill,
                        continuous,
                    }
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
                    SlashCommandType::LinkChannel {
                        discord_channel_id: interaction.channel_id.into(),
                        lamprey_channel_id: channel_id,
                        backfill,
                    }
                }
                _ => return Err(anyhow::anyhow!("unknown link subcommand")),
            }
        }
        "unlink" => {
            let subcommand = interaction
                .data
                .options
                .get(0)
                .ok_or_else(|| anyhow::anyhow!("missing subcommand"))?;
            match subcommand.name.as_str() {
                "guild" => SlashCommandType::UnlinkGuild {
                    discord_guild_id: interaction
                        .guild_id
                        .ok_or_else(|| anyhow::anyhow!("not in guild"))?,
                },
                "channel" => SlashCommandType::UnlinkChannel {
                    discord_channel_id: interaction.channel_id.into(),
                },
                _ => return Err(anyhow::anyhow!("unknown unlink subcommand")),
            }
        }
        _ => return Err(anyhow::anyhow!("unknown command")),
    };

    Ok(SlashCommand { interaction, inner })
}
