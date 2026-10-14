use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "scanner-nsfw")]
#[command(about = "NSFW image scanner", long_about = None)]
pub struct Cli {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE", default_value = "scanner-nsfw.toml")]
    pub config: PathBuf,
}
