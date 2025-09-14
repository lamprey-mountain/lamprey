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

    /// garbage collect media deleted over a week ago
    GcMedia {},

    /// garbage collect messages deleted over a week ago
    GcMessages {},

    /// garbage collect expired sessions
    GcSession {},

    /// run all garbage collection routines
    GcAll {},
}
