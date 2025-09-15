use std::path::PathBuf;

use clap::{Parser, Subcommand};
use common::v1::types::UserId;

/// Simple program to greet a person
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// start the main server
    Serve {},

    /// check config
    Check {},

    /// garbage collect media deleted over a week ago
    GcMedia {},

    /// garbage collect messages deleted over a week ago
    GcMessages {},

    /// garbage collect expired sessions
    GcSession {},

    /// run all garbage collection routines
    GcAll {},

    /// upgrade a guest to a registered user
    Register {
        user_id: UserId,

        /// audit log reason why this user was manually registered
        #[arg(short, long)]
        reason: Option<String>,
    },

    /// make a user an admin
    MakeAdmin {
        user_id: UserId,

        /// make this user a full administrator, otherwise just invite to the server room
        #[arg(short, long)]
        full: bool,
    },
}
