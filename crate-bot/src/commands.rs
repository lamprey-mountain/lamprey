//! commands and args for the bot

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// ping the bot to see if its online
    Ping,

    /// commands for voice/music management
    #[command(subcommand, alias = "vc")]
    Voice(VoiceCommand),

    /// reminder commands
    #[command(subcommand)]
    Remind(RemindCommand),
    // /// llm commands (TODO?)
    // #[command(subcommand)]
    // Llm(LlmCommand),
}

#[derive(Debug, clap::Subcommand)]
pub enum VoiceCommand {
    /// join the voice thread you're in
    Join,

    /// leave the voice thread the bot is in
    Leave,

    /// play or resume music
    Play,

    /// toggle pause state
    Pause {
        #[arg(short)]
        paused: Option<bool>,
    },

    /// stop current music
    Stop,
}

#[derive(Debug, clap::Subcommand)]
pub enum RemindCommand {
    /// add a reminder (format: [duration] [text...])
    Add {
        /// duration (e.g., "5m", "1h", "2d", "5m30s")
        duration: String,
        /// reminder text
        text: Vec<String>,
    },

    /// remove a reminder by id
    Remove {
        /// reminder id
        id: u64,
    },

    /// remove all reminders
    RemoveAll,

    /// list all reminders
    List,
}
