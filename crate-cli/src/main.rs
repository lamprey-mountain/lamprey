use std::{fmt, io::IsTerminal};

use anyhow::{Result, bail};
use clap::Parser;
use demand::Confirm;
use lamprey_cli::args::{Args, AuthCommand, ChannelCommand, Command, MediaCommand, MessageCommand};
use lamprey_cli::auth;
use lamprey_cli::channel;
use lamprey_cli::config::LyConfig;
use lamprey_cli::events;
use lamprey_cli::media;
use lamprey_cli::message;
use serde::Serialize;

/// output of a command
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Output {
    Login(auth::LoginResult),
    Logout(auth::LogoutResult),
    Check(Box<auth::CheckResult>),
    List(auth::ListResult),
    SetDefault(auth::SetDefaultResult),
    Message(message::MessageResult),
    Delete(message::DeleteResult),
    Channel(channel::ChannelResult),
    ChannelRemove(channel::RemoveResult),
    Media(media::MediaResult),
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Output::Login(result) => fmt::Display::fmt(result, f),
            Output::Logout(result) => fmt::Display::fmt(result, f),
            Output::Check(result) => fmt::Display::fmt(result, f),
            Output::List(result) => fmt::Display::fmt(result, f),
            Output::SetDefault(result) => fmt::Display::fmt(result, f),
            Output::Message(result) => fmt::Display::fmt(result, f),
            Output::Delete(result) => fmt::Display::fmt(result, f),
            Output::Channel(result) => fmt::Display::fmt(result, f),
            Output::ChannelRemove(result) => fmt::Display::fmt(result, f),
            Output::Media(result) => fmt::Display::fmt(result, f),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let Args {
        config,
        login,
        json,
        verbose,
        command,
    } = Args::parse();
    let login = login.as_deref();
    let mut config = LyConfig::open(config)?;

    let output = match command {
        Command::Auth { command } => match command {
            AuthCommand::Login {
                api_url,
                token,
                name,
                register,
                scope,
                registration_token,
            } => Output::Login(
                auth::login(
                    &mut config,
                    api_url,
                    token,
                    name,
                    register,
                    &scope,
                    registration_token.as_ref(),
                    verbose,
                )
                .await?,
            ),
            AuthCommand::Logout { all, yes } => {
                if all && login.is_some() {
                    bail!("--all cannot be combined with --login");
                }

                let action = if all {
                    "log out of all stored logins"
                } else {
                    "log out"
                };
                confirm(yes, action)?;

                Output::Logout(auth::logout(&mut config, login, all).await?)
            }
            AuthCommand::Check => Output::Check(Box::new(auth::check(&mut config, login).await?)),
            AuthCommand::List => Output::List(auth::list(&config)),
            AuthCommand::SetDefault { name } => {
                Output::SetDefault(auth::set_default(&mut config, &name)?)
            }
        },
        Command::Message { command } => match command {
            MessageCommand::Send {
                channel_id,
                content,
                reply_to,
            } => Output::Message(
                message::send(&mut config, login, channel_id, content, reply_to).await?,
            ),
            MessageCommand::Get {
                channel_id,
                message_id,
            } => Output::Message(message::get(&mut config, login, channel_id, message_id).await?),
            MessageCommand::Delete {
                channel_id,
                message_id,
                yes,
            } => {
                confirm(yes, "delete this message")?;

                Output::Delete(message::delete(&mut config, login, channel_id, message_id).await?)
            }
        },
        Command::Channel { command } => match command {
            ChannelCommand::Create {
                room_id,
                name,
                kind,
            } => Output::Channel(channel::create(&mut config, login, room_id, name, kind).await?),
            ChannelCommand::Get { channel_id } => {
                Output::Channel(channel::get(&mut config, login, channel_id).await?)
            }
            ChannelCommand::Update {
                channel_id,
                name,
                description,
            } => Output::Channel(
                channel::update(&mut config, login, channel_id, name, description).await?,
            ),
            ChannelCommand::Remove { channel_id, yes } => {
                confirm(yes, "remove this channel")?;

                Output::ChannelRemove(channel::remove(&mut config, login, channel_id).await?)
            }
        },
        Command::Media { command } => match command {
            MediaCommand::Upload {
                path,
                alt,
                strip_exif,
            } => Output::Media(media::upload(&mut config, login, &path, alt, strip_exif).await?),
            MediaCommand::Get { media_id } => {
                Output::Media(media::get(&mut config, login, media_id).await?)
            }
        },
        Command::Events {
            channel,
            room,
            event,
        } => {
            events::tail(&mut config, login, &channel, &room, &event, json, verbose).await?;
            return Ok(());
        }
    };

    emit(json, &output)
}

/// confirm a destructive command, unless `--yes` was passed
fn confirm(yes: bool, action: &str) -> Result<()> {
    if yes {
        return Ok(());
    }

    if !std::io::stdin().is_terminal() {
        bail!("refusing to {action} without --yes");
    }

    if Confirm::new(format!("really {action}?")).run()? {
        Ok(())
    } else {
        bail!("aborted")
    }
}

fn emit<T: Serialize + fmt::Display>(json: bool, value: &T) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{value}");
    }

    Ok(())
}
