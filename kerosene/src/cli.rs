use std::path::PathBuf;

use clap::{Parser, Subcommand};
use lamprey::v1::types::{ChannelId, MediaId, MessageId, RedexId};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    // /// Token to use for authenticating to the api
    // #[arg(short, long)]
    // pub token: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// set up a new server
    Init,

    /// print resolved config
    Config,

    /// start the server
    Serve {
        // TODO: control what gets served
    },

    /// run healthchecks
    Health {
        /// check the health of a running server
        #[arg(short, long, default_value = "false")]
        online: bool,
    },
    // NOTE: do i add administration or maintenence commands here?
    // TODO: add commands to create a user, promote a user to admin
    // maybe make the init subcommand interactive?
}
