use std::path::PathBuf;

use clap::{Parser, Subcommand};
use common::v1::types::{ChannelId, MediaId, MessageId, RedexId};

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
    Redex {
        #[command(subcommand)]
        command: RedexCommand,
    },

    /// tail the event stream
    Events {
        // TODO: resuming
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// login with the api
    Login,

    /// logout and clear stored credentials
    Logout,
}

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// send a message
    Send {
        /// id of the channel to send to
        #[arg(short, long)]
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
        message_id: String,
    },

    /// delete a message
    Delete {
        /// id of the channel
        channel_id: ChannelId,

        /// id of the message
        message_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ChannelCommand {
    /// create a new channel
    Create {
        // TODO: room_id
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
    },
}

#[derive(Debug, Subcommand)]
pub enum MediaCommand {
    /// upload media content
    Upload {
        /// path to the file to upload
        path: PathBuf,
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
