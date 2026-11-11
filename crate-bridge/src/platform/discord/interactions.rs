use twilight_model::application::command::{
    Command, CommandOption, CommandOptionType, CommandType,
};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::{Interaction, InteractionContextType, InteractionData};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;

use crate::prelude::*;

/// a slash command from discord
#[derive(Debug)]
pub struct SlashCommand {
    pub interaction: Interaction,
    pub inner: SlashCommandType,
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

fn option(kind: CommandOptionType, name: &str, description: &str) -> CommandOption {
    CommandOption {
        autocomplete: None,
        channel_types: None,
        choices: None,
        description: description.to_owned(),
        description_localizations: None,
        kind,
        max_length: None,
        max_value: None,
        min_length: None,
        min_value: None,
        name: name.to_owned(),
        name_localizations: None,
        options: None,
        file_types: None,
        required: None,
    }
}

// `Command` has no `Default` impl and its `dm_permission` field is deprecated
// in favour of `contexts`, so the whole builder needs the lint silenced
#[allow(deprecated)]
fn command(name: &str, description: &str, options: Vec<CommandOption>) -> Command {
    Command {
        application_id: None,
        contexts: Some(vec![InteractionContextType::Guild]),
        default_member_permissions: None,
        description: description.to_owned(),
        description_localizations: None,
        dm_permission: None,
        guild_id: None,
        id: None,
        integration_types: None,
        kind: CommandType::ChatInput,
        name: name.to_owned(),
        name_localizations: None,
        nsfw: None,
        options,
        version: Id::new(1),
    }
}

/// get discord slash commands
pub fn get_commands(disable_permission_checks: bool) -> Vec<Command> {
    let permissions = if disable_permission_checks {
        None
    } else {
        Some(Permissions::from_bits_truncate(536870944))
    };

    let mut ping = command("ping", "check if the bridge is alive", vec![]);
    ping.default_member_permissions = permissions;

    let mut guild_room_id =
        option(CommandOptionType::String, "room_id", "the uuid of the room to link to");
    guild_room_id.required = Some(true);

    let guild_backfill = option(
        CommandOptionType::Boolean,
        "backfill",
        "whether to clone the full history of every channel",
    );

    let guild_continuous = option(
        CommandOptionType::Boolean,
        "continuous",
        "whether to create new portals as channels and threads are created (this is bidirectional)",
    );

    let mut guild = option(CommandOptionType::SubCommand, "guild", "link this guild (server)");
    guild.options = Some(vec![guild_room_id, guild_backfill, guild_continuous]);

    let mut channel_channel_id =
        option(CommandOptionType::String, "channel_id", "the uuid of the channel to link to");
    channel_channel_id.required = Some(true);

    let channel_backfill = option(
        CommandOptionType::Boolean,
        "backfill",
        "whether to clone the full history of this channel",
    );

    let mut channel = option(CommandOptionType::SubCommand, "channel", "link this channel");
    channel.options = Some(vec![channel_channel_id, channel_backfill]);

    let mut link = command("link", "link something to lamprey", vec![guild, channel]);
    link.default_member_permissions = permissions;

    let unlink_guild = option(
        CommandOptionType::SubCommand,
        "guild",
        "unlink this guild (server)",
    );
    let unlink_channel =
        option(CommandOptionType::SubCommand, "channel", "unlink this channel");

    let mut unlink = command(
        "unlink",
        "unlink something from lamprey",
        vec![unlink_guild, unlink_channel],
    );
    unlink.default_member_permissions = permissions;

    // TODO: command(s) to edit an existing realm/portal
    // TODO: command(s) to moderate (kick, ban, timeout) users on other platforms

    vec![ping, link, unlink]
}

pub fn parse_interaction(interaction: Interaction) -> Result<SlashCommand> {
    let data: &CommandData = match interaction.data.as_ref() {
        Some(InteractionData::ApplicationCommand(data)) => data,
        _ => return Err(anyhow::anyhow!("not an application command")),
    };

    let inner = match data.name.as_str() {
        "ping" => SlashCommandType::Ping,
        "link" => {
            let subcommand = data
                .options
                .first()
                .ok_or_else(|| anyhow::anyhow!("missing subcommand"))?;
            let CommandOptionValue::SubCommand(options) = &subcommand.value else {
                return Err(anyhow::anyhow!("invalid subcommand"));
            };

            match subcommand.name.as_str() {
                "guild" => {
                    let mut room_id_str = None;
                    let mut backfill = false;
                    let mut continuous = false;
                    for opt in options {
                        match opt.name.as_str() {
                            "room_id" => {
                                if let CommandOptionValue::String(s) = &opt.value {
                                    room_id_str = Some(s.to_owned());
                                }
                            }
                            "backfill" => {
                                if let CommandOptionValue::Boolean(b) = opt.value {
                                    backfill = b;
                                }
                            }
                            "continuous" => {
                                if let CommandOptionValue::Boolean(b) = opt.value {
                                    continuous = b;
                                }
                            }
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
                                if let CommandOptionValue::String(s) = &opt.value {
                                    channel_id_str = Some(s.to_owned());
                                }
                            }
                            "backfill" => {
                                if let CommandOptionValue::Boolean(b) = opt.value {
                                    backfill = b;
                                }
                            }
                            _ => {}
                        }
                    }
                    let channel_id = channel_id_str
                        .ok_or_else(|| anyhow::anyhow!("missing channel_id"))?
                        .parse()?;
                    let discord_channel_id = interaction
                        .channel
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("not in channel"))?
                        .id;
                    SlashCommandType::LinkChannel {
                        discord_channel_id,
                        lamprey_channel_id: channel_id,
                        backfill,
                    }
                }
                _ => return Err(anyhow::anyhow!("unknown link subcommand")),
            }
        }
        "unlink" => {
            let subcommand = data
                .options
                .first()
                .ok_or_else(|| anyhow::anyhow!("missing subcommand"))?;
            match subcommand.name.as_str() {
                "guild" => SlashCommandType::UnlinkGuild {
                    discord_guild_id: interaction
                        .guild_id
                        .ok_or_else(|| anyhow::anyhow!("not in guild"))?,
                },
                "channel" => SlashCommandType::UnlinkChannel {
                    discord_channel_id: interaction
                        .channel
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("not in channel"))?
                        .id,
                },
                _ => return Err(anyhow::anyhow!("unknown unlink subcommand")),
            }
        }
        _ => return Err(anyhow::anyhow!("unknown command")),
    };

    Ok(SlashCommand { interaction, inner })
}
