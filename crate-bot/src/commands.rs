use crate::{bot::Bot, prelude::*};
use common::v1::types::{Message, MessageCreate, misc::duration::Duration, util::Time};
use sdk::voice::{FileSource, Player};

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
        id: i64,
    },

    /// remove all reminders
    RemoveAll,

    /// list all reminders
    List,
}

impl Bot {
    // TODO: -> Result<Option<MessageCreate>>
    pub(crate) async fn handle_command(
        &mut self,
        message: &Message,
        cmd: Command,
    ) -> Result<String> {
        let resp = match cmd {
            Command::Ping => "pong!".to_string(),
            Command::Voice(v) => match v {
                VoiceCommand::Join => {
                    if self.voice.is_some() {
                        "already joined".to_string()
                    } else {
                        let voice = self.client.voice(message.channel_id).await?;
                        let (player, producer) = Player::new_audio();
                        voice.publish(producer).await?;
                        self.player = Some(player);
                        self.voice = Some(voice);
                        "joined".to_string()
                    }
                }
                VoiceCommand::Leave => {
                    if let Some(voice) = self.voice.take() {
                        if let Some(player) = self.player.take() {
                            player.stop();
                        }
                        voice.disconnect().await?;
                        "left".to_string()
                    } else {
                        "not joined".to_string()
                    }
                }
                VoiceCommand::Play => {
                    if let Some(player) = self.player.clone() {
                        player.set_paused(false);
                        "playing".to_string()
                    } else {
                        let voice = match self.voice.clone() {
                            Some(voice) => voice,
                            None => {
                                let voice = self.client.voice(message.channel_id).await?;
                                self.voice = Some(voice.clone());
                                voice
                            }
                        };
                        let source = FileSource::from_path(&self.config.music_path)?;
                        let (player, producer) = Player::new_audio();
                        player.play(source);
                        voice.publish(producer).await?;
                        self.player = Some(player);
                        "playing".to_string()
                    }
                }
                VoiceCommand::Pause { paused } => match &self.player {
                    Some(player) => {
                        let paused = paused.unwrap_or(!player.paused());
                        player.set_paused(paused);
                        if paused { "paused" } else { "resumed" }.to_string()
                    }
                    None => "no player".to_string(),
                },
                VoiceCommand::Stop => {
                    if let Some(player) = self.player.take() {
                        player.stop();
                        "stopped".to_string()
                    } else {
                        "no player".to_string()
                    }
                }
            },
            Command::Remind(cmd) => match cmd {
                RemindCommand::Add { duration, text } => {
                    let text = text.join(" ");
                    let duration: Duration = duration
                        .parse()
                        .map_err(|_| anyhow::anyhow!("invalid duration"))?;
                    let scheduled_at = (Time::now_utc() + duration).to_string();
                    self.db
                        .add_reminder(message.author_id, &text, &scheduled_at)
                        .await?;
                    format!("Reminder set for {scheduled_at}: {text}")
                }
                RemindCommand::Remove { id } => {
                    self.db.remove_reminder(id).await?;
                    format!("Removed reminder {id}")
                }
                RemindCommand::RemoveAll => {
                    self.db.remove_all_reminders(message.author_id).await?;
                    "Removed all your reminders".to_string()
                }
                RemindCommand::List => {
                    let reminders = self.db.list_reminders(message.author_id).await?;

                    if reminders.is_empty() {
                        "No reminders".to_string()
                    } else {
                        let mut output = String::new();
                        for reminder in &reminders {
                            output
                                .push_str(&format!("[{}] {}", reminder.id, reminder.scheduled_at));
                            if !reminder.text.is_empty() {
                                output.push_str(&format!(": {}", reminder.text));
                            }
                            output.push('\n');
                        }
                        output
                    }
                }
            },
        };
        Ok(resp)
    }
}
