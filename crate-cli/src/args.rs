use std::path::PathBuf;

use clap::{Parser, Subcommand};
use common::v1::types::{ChannelId, MediaId, MessageId, RedexId, SessionToken, oauth::Scope};
use url::Url;

/// tool to interact with the lamprey api
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Login to use for authenticating to the api
    #[arg(short, long)]
    pub login: Option<String>,

    /// Output machine readable json
    #[arg(short, long, global = true)]
    pub json: bool,

    /// Print progress and diagnostic output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// tools for managing auth
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },

    /// tools for managing messages
    Message {
        #[command(subcommand)]
        command: MessageCommand,
    },

    /// tools for managing channels
    Channel {
        #[command(subcommand)]
        command: ChannelCommand,
    },

    /// tools for managing media
    Media {
        #[command(subcommand)]
        command: MediaCommand,
    },

    /// tools for managing redexes
    #[cfg(any())] // TODO: work on this once redexes become more usable
    Redex {
        #[command(subcommand)]
        command: RedexCommand,
    },

    /// tail the event stream
    Events {
        // TODO: tail everything, room, channel, filter by event type?
        // TODO: resuming
        // TODO: output as pretty, ndjson
        // TODO: maybe handle stdin in some way?
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// login with the api
    Login {
        /// api url to log in to
        #[arg(short = 'u', long)]
        api_url: Option<Url>,

        /// session token to validate and store instead of authenticating
        #[arg(short = 't', long, value_parser = parse_session_token)]
        token: Option<SessionToken>,

        /// name to store this login under
        name: Option<String>,

        /// register a dynamic oauth client and log in through the browser
        #[arg(long)]
        register: bool,

        /// space-separated scopes to request when registering a dynamic client
        #[arg(
            short = 's',
            long,
            value_delimiter = ' ',
            default_value = "full",
            value_parser = parse_scope,
        )]
        scope: Vec<Scope>,

        /// registration token, for servers that require one
        #[arg(long, value_parser = parse_session_token)]
        registration_token: Option<SessionToken>,
    },

    /// logout and revoke stored credentials
    Logout {
        /// log out of all stored logins
        #[arg(long)]
        all: bool,

        /// skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// check if a stored login is still valid, and print its metadata
    Check,

    /// list stored logins
    List,

    /// set the default login
    SetDefault {
        /// name of the login to make the default
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// send a message
    Send {
        /// id of the channel to send to
        channel_id: ChannelId,

        /// message content
        #[arg(short, long)]
        content: String,

        /// reply to message id
        #[arg(short, long)]
        reply_to: Option<MessageId>,
        // TODO: embeds, components, attachments
    },

    /// get a specific message
    Get {
        /// id of the channel
        channel_id: ChannelId,

        /// id of the message
        message_id: MessageId,
    },

    /// delete a message
    Delete {
        /// id of the channel
        channel_id: ChannelId,

        /// id of the message
        message_id: MessageId,

        /// skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    // TODO: audit reason for delete
    // TODO: fetch multiple messages
    // TODO: edit message
}

#[derive(Debug, Subcommand)]
pub enum ChannelCommand {
    /// create a new channel
    Create {
        // TODO: room_id, description, etc
        // TODO: support creating threads with this(?)
        /// name of the channel
        #[arg(short, long)]
        name: String,

        /// channel type (text, voice, etc.)
        #[arg(short, long)]
        kind: String, // TODO: use enum
    },

    /// get channel details
    Get {
        /// id of the channel
        channel_id: ChannelId,
    },

    /// update channel properties
    Update {
        /// id of the channel
        channel_id: ChannelId,

        /// new name for the channel
        #[arg(short, long)]
        name: Option<String>,

        /// new description for the channel
        #[arg(short, long)]
        description: Option<String>,
    },

    /// remove a channel
    Remove {
        /// id of the channel
        channel_id: ChannelId,

        /// skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum MediaCommand {
    /// upload media content
    Upload {
        /// path to the file to upload
        path: PathBuf,
        // TODO: flags to set alt text and stuff
    },

    /// get media metadata
    Get {
        /// id of the media
        media_id: MediaId,
    },
}

#[derive(Debug, Subcommand)]
pub enum RedexCommand {
    /// create a new redex
    Create {
        /// channel id to create redex in
        #[arg(short, long)]
        channel_id: ChannelId,

        /// media id of redex source
        #[arg(short, long)]
        media_id: MediaId,
    },

    /// get redex metadata
    Get {
        /// id of the redex to get
        redex_id: RedexId,
    },

    /// manually trigger a redex
    Trigger {
        /// id of the redex to run
        redex_id: RedexId,

        /// id of the trigger
        trigger_id: String,
    },
}

fn parse_scope(value: &str) -> Result<Scope, String> {
    value
        .parse()
        .map_err(|()| format!("unknown scope `{value}`"))
}

fn parse_session_token(value: &str) -> Result<SessionToken, std::convert::Infallible> {
    Ok(SessionToken::from(value.to_owned()))
}
