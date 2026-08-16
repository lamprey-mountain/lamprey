use crate::{bot::Bot, prelude::*};
use common::v1::types::{Message, MessageCreate, misc::duration::Duration, util::Time};

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
                    // TODO
                    "joined".to_string()
                }
                VoiceCommand::Leave => {
                    // TODO
                    "left".to_string()
                }
                VoiceCommand::Play => {
                    // new code:
                    // let voice = self.client.voice(message.channel_id).connect().await?;

                    // old code:
                    // let _ = self.join_voice(message).await;
                    // if let Some(p) = &*self.player.lock().await {
                    //     p.send(PlayerCommand::Play(self.config.music_path.clone().into()))
                    //         .await?;
                    //     "playing".to_string()
                    // } else {
                    //     "no player".to_string()
                    // }
                    todo!()
                }
                VoiceCommand::Pause { paused } => {
                    // if let Some(p) = &*self.player.lock().await {
                    //     p.send(PlayerCommand::Pause(paused)).await?;
                    //     "(un)paused".to_string()
                    // } else {
                    //     "no player".to_string()
                    // }
                    todo!()
                }
                VoiceCommand::Stop => {
                    // if let Some(p) = &*self.player.lock().await {
                    //     p.send(PlayerCommand::Stop).await?;
                    //     "stopped".to_string()
                    // } else {
                    //     "no player".to_string()
                    // }
                    todo!()
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
