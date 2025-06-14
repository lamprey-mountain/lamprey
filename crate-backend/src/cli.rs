use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

    /// send a test email
    TestMail {
        #[arg(short, long)]
        to: String,
    },
    // TODO
    // /// admin server management
    // Admin {},

    // /// start a syncing node
    // ServeSyncer {},

    // /// start a voip node
    // ServeVoip {},

    // /// start a media processing node
    // ServeMedia {},
}
