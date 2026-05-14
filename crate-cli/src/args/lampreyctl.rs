use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use common::v1::types::{RoomId, UserId};

/// tool to control lamprey servers
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Token to use for authenticating to the api
    #[arg(short, long)]
    pub token: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// start a server
    Serve {
        #[command(subcommand)]
        target: ServeCommand,
    },

    /// various maintenence tasks
    Maintenence {
        #[command(subcommand)]
        target: MaintenenceCommand,
    },

    /// run healthchecks
    Check,

    /// manage users
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    // admin command
    // check config
    // migrate
}

#[derive(Debug, Subcommand)]
pub enum ServeCommand {
    /// start the main api server
    Api,

    /// start the media proxy server
    Media {
        #[arg(short, long, default_value = "media.toml")]
        media_config: PathBuf,
    },

    /// start the voice server
    Voice {
        #[arg(short, long, default_value = "sfu.toml")]
        sfu_config: PathBuf,
    },
    // media scanner server?
}

#[derive(Debug, Subcommand)]
pub enum MaintenenceCommand {
    /// garbage collect old data
    Gc {
        #[arg(short, long)]
        target: Vec<GcTarget>,

        #[arg(short, long)]
        mode: GcMode,

        #[arg(short = 'a', long = "async")]
        run_async: bool,
    },

    /// purge caches
    PurgeCache {
        #[arg(short, long)]
        target: Vec<CacheTarget>,
    },

    /// manage search indexes
    SearchIndex {
        #[command(subcommand)]
        command: SearchIndexCommand,
    },

    /// unload a room
    UnloadRoom { room_id: RoomId },

    /// unload then load a room
    ReloadRoom { room_id: RoomId },

    /// broadcast a message to everyone
    Broadcast {
        // TODO
    },

    /// whisper a message to a specific user
    Whisper {
        // TODO
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum GcTarget {
    Media,
    Messages,
    Session,
    AuditLog,
    RoomAnalytics,
    All,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, ValueEnum)]
pub enum GcMode {
    /// set a flag in the db for deleted items
    Mark,

    /// actually delete data
    Sweep,

    /// do full as a dry run
    Dry,

    /// mark and sweep
    #[default]
    Full,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum CacheTarget {
    Channels,
    Embeds,
    Permissions,
    Rooms,
    Sessions,
    Users,
    All,
}

#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// create a new user
    Create {
        /// the name of the new user
        #[arg(short, long)]
        name: String,
    },

    /// upgrade a guest user to a registered user
    Register {
        /// audit log reason why this user was manually registered
        #[arg(short, long)]
        reason: Option<String>,

        /// the user to register
        user_id: UserId,
    },

    /// make a user an admin
    MakeAdmin {
        /// the user to make an admin
        user_id: UserId,
    },
}

// TODO
#[derive(Debug, Subcommand)]
pub enum SearchIndexCommand {
    ReindexChannel,
    ReindexEverything,
    ReindexRoom,
    DlqList,
    DlqDelete,
    DeqRetry,
    Stats,
}
